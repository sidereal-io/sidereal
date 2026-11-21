use super::*;
use crate::{
    database::initialize_with_storage,
    storage::{
        objects::{Hook, ObjectStore, Step},
        tests::TempRoot,
    },
    test_support::TestDatabase,
};
use sidereal_plugin_abi::{AssetContext, AssetProposal, Pack, Source, SourceError};
use sqlx::{Connection, Executor, PgConnection};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Mutex},
    time::Duration,
};

const SECONDS: Duration = Duration::from_secs(3);
const ABC: &str = "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

/// A Source registered through the public contract. It proposes an image
/// for bytes starting with `a`, refuses `r…`, fails `f…`, and records the
/// bytes it saw.
struct Recorder {
    seen: Arc<Mutex<Vec<u8>>>,
}

impl Source for Recorder {
    fn id(&self) -> &str {
        "test.recorder"
    }
    fn version(&self) -> &str {
        "9.9.9"
    }
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), SourceError> {
        let mut bytes = vec![0; context.candidate_size() as usize];
        let n = context
            .read_candidate(0, &mut bytes)
            .map_err(|_| SourceError::Failed("read".into()))?;
        self.seen.lock().unwrap().extend_from_slice(&bytes[..n]);
        match bytes.first() {
            Some(b'r') => Err(SourceError::Refused("not for this source".into())),
            Some(b'f') => Err(SourceError::Failed("broken".into())),
            _ => context
                .propose_asset(AssetProposal::new("image", "image/x-test"))
                .map_err(|_| SourceError::Failed("proposal".into())),
        }
    }
}

struct TestPack(Arc<Mutex<Vec<u8>>>);
impl Pack for TestPack {
    fn id(&self) -> &str {
        "test"
    }
    fn register(&self, registry: &mut Registry) {
        registry.register_source(Box::new(Recorder {
            seen: self.0.clone(),
        }));
    }
}

struct Fixture {
    db: TestDatabase,
    temp: TempRoot,
    database: crate::database::Database,
    paired: Uuid,
    registry: Arc<Registry>,
    seen: Arc<Mutex<Vec<u8>>>,
}

impl Fixture {
    async fn new() -> Self {
        let db = TestDatabase::new();
        let temp = TempRoot::new();
        let (database, paired) =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &temp.root())
                .await
                .unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut registry = Registry::new();
        TestPack(seen.clone()).register(&mut registry);
        Self {
            db,
            temp,
            database,
            paired,
            registry: Arc::new(registry),
            seen,
        }
    }

    fn objects(&self, hook: Option<Hook>) -> ObjectStore {
        ObjectStore::open_with(&self.temp.root(), self.paired, hook).unwrap()
    }

    /// Stage `bytes` and ingest them, as the upload route does.
    async fn upload_with(
        &self,
        objects: &ObjectStore,
        name: &str,
        bytes: &[u8],
        cancelled: bool,
    ) -> Result<AssetRecord, IngestError> {
        let mut staging = objects.begin().map_err(IngestError::Storage)?;
        staging.write(bytes).map_err(|e| match e {
            crate::storage::objects::StageError::Storage(e) => IngestError::Storage(e),
            other => panic!("{other:?}"),
        })?;
        let staged = staging.finish().map_err(IngestError::Storage)?;
        ingest(
            &self.database,
            objects,
            self.registry.clone(),
            "test.recorder",
            name,
            staged,
            Arc::new(AtomicBool::new(cancelled)),
        )
        .await
    }

    async fn upload(&self, name: &str, bytes: &[u8]) -> Result<AssetRecord, IngestError> {
        self.upload_with(&self.objects(None), name, bytes, false)
            .await
    }

    async fn failing_at(&self, failing: Step, bytes: &[u8]) -> Result<AssetRecord, IngestError> {
        let hook: Hook = Arc::new(move |step| {
            if step == failing {
                Err(std::io::Error::other("injected"))
            } else {
                Ok(())
            }
        });
        self.upload_with(&self.objects(Some(hook)), "nebula.png", bytes, false)
            .await
    }

    async fn conn(&self) -> PgConnection {
        PgConnection::connect_with(&self.db.options).await.unwrap()
    }

    /// Asset rows plus Asset version rows.
    async fn rows(&self) -> i64 {
        sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM public.assets) + (SELECT count(*) FROM public.asset_versions)",
        )
        .fetch_one(&mut self.conn().await)
        .await
        .unwrap()
    }

    fn object(&self, hash: &str) -> PathBuf {
        self.temp
            .path
            .join("store")
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(hash)
    }

    fn staging_files(&self) -> usize {
        std::fs::read_dir(self.temp.path.join("tmp")).map_or(0, |entries| entries.count())
    }
}

fn listing(path: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

mod ingest_commit {
    use super::*;

    #[tokio::test]
    async fn an_accepted_upload_commits_one_asset_pointing_to_its_version() {
        let fixture = Fixture::new().await;
        let record = fixture.upload("nebula.png", b"abc").await.unwrap();
        assert_eq!(
            (record.name.as_str(), record.kind.as_str()),
            ("nebula.png", "image")
        );
        assert_eq!(record.version.content_hash, ABC);
        assert_eq!(record.version.byte_size, 3);
        assert_eq!(record.version.media_type, "image/x-test");
        assert_eq!(record.current_version_id, record.version.id);
        assert_eq!(fixture.rows().await, 2);
    }

    #[tokio::test]
    async fn a_committed_asset_reads_back_from_the_database() {
        let fixture = Fixture::new().await;
        let record = fixture.upload("nebula.png", b"abc").await.unwrap();
        assert_eq!(
            fixture.database.asset(record.id).await.unwrap(),
            Some(record.clone())
        );
        assert_eq!(fixture.database.assets().await.unwrap(), [record]);
    }

    #[tokio::test]
    async fn the_object_is_published_before_the_commit() {
        let fixture = Fixture::new().await;
        let record = fixture.upload("nebula.png", b"abc").await.unwrap();
        assert_eq!(
            std::fs::read(fixture.object(&record.version.content_hash)).unwrap(),
            b"abc"
        );
        assert_eq!(fixture.staging_files(), 0);
    }

    #[tokio::test]
    async fn the_ids_are_fresh_uuids_not_the_name_or_hash() {
        let fixture = Fixture::new().await;
        let record = fixture.upload("nebula.png", b"abc").await.unwrap();
        assert_ne!(record.id, record.version.id);
        for id in [record.id, record.version.id] {
            assert_eq!(id.get_version_num(), 4);
            assert!(!ABC.contains(&id.simple().to_string()[..8]));
        }
    }

    #[tokio::test]
    async fn provenance_names_the_registered_source_that_ran() {
        let fixture = Fixture::new().await;
        let record = fixture.upload("nebula.png", b"abc").await.unwrap();
        assert_eq!(*fixture.seen.lock().unwrap(), b"abc");
        assert_eq!(
            (
                record.version.source_id.as_str(),
                record.version.source_version.as_str()
            ),
            ("test.recorder", "9.9.9")
        );
    }

    #[tokio::test]
    async fn a_refused_candidate_commits_nothing_and_publishes_nothing() {
        let fixture = Fixture::new().await;
        let outcome = fixture.upload("nebula.png", b"rejected").await;
        assert!(
            matches!(outcome, Err(IngestError::Refused(_))),
            "{outcome:?}"
        );
        assert_eq!(fixture.rows().await, 0);
        assert!(!fixture.temp.path.join("store").exists());
        assert_eq!(fixture.staging_files(), 0);
    }

    #[tokio::test]
    async fn a_failed_source_commits_nothing() {
        let fixture = Fixture::new().await;
        let outcome = fixture.upload("nebula.png", b"fails").await;
        assert!(
            matches!(outcome, Err(IngestError::Source(_))),
            "{outcome:?}"
        );
        assert_eq!(fixture.rows().await, 0);
    }

    #[tokio::test]
    async fn an_unknown_source_commits_nothing() {
        let fixture = Fixture::new().await;
        let objects = fixture.objects(None);
        let mut staging = objects.begin().unwrap();
        staging.write(b"abc").unwrap();
        let outcome = ingest(
            &fixture.database,
            &objects,
            fixture.registry.clone(),
            "test.missing",
            "nebula.png",
            staging.finish().unwrap(),
            Arc::new(AtomicBool::new(false)),
        )
        .await;
        assert!(
            matches!(outcome, Err(IngestError::Source(_))),
            "{outcome:?}"
        );
        assert_eq!(fixture.rows().await, 0);
    }

    #[tokio::test]
    async fn a_failed_write_commits_nothing() {
        let fixture = Fixture::new().await;
        assert!(fixture
            .failing_at(Step::WriteStaging, b"abc")
            .await
            .is_err());
        assert_eq!(fixture.rows().await, 0);
    }

    #[tokio::test]
    async fn a_failed_file_sync_commits_nothing() {
        let fixture = Fixture::new().await;
        let outcome = fixture.failing_at(Step::SyncStaging, b"abc").await;
        assert!(
            matches!(outcome, Err(IngestError::Storage(_))),
            "{outcome:?}"
        );
        assert_eq!(fixture.rows().await, 0);
        assert!(!fixture.object(ABC).exists());
    }

    #[tokio::test]
    async fn a_failed_folder_sync_commits_nothing_and_keeps_the_object() {
        let fixture = Fixture::new().await;
        let outcome = fixture
            .failing_at(
                Step::SyncDirectory(crate::storage::objects::Level::Root),
                b"abc",
            )
            .await;
        assert!(
            matches!(outcome, Err(IngestError::Storage(_))),
            "{outcome:?}"
        );
        assert_eq!(fixture.rows().await, 0);
        assert_eq!(std::fs::read(fixture.object(ABC)).unwrap(), b"abc");
    }

    /// Make every Asset version insert fail, after the Asset row went in.
    async fn break_version_inserts(fixture: &Fixture) {
        fixture
            .conn()
            .await
            .execute(
                "CREATE FUNCTION public.refuse() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected'; END $$;
                 CREATE TRIGGER refuse BEFORE INSERT ON public.asset_versions FOR EACH ROW EXECUTE FUNCTION public.refuse();",
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_failed_commit_leaves_no_partial_asset_and_keeps_the_object() {
        let fixture = Fixture::new().await;
        break_version_inserts(&fixture).await;
        let outcome = fixture.upload("nebula.png", b"abc").await;
        assert!(
            matches!(outcome, Err(IngestError::Database(_))),
            "{outcome:?}"
        );
        assert_eq!(fixture.rows().await, 0);
        assert_eq!(std::fs::read(fixture.object(ABC)).unwrap(), b"abc");
    }

    #[tokio::test]
    async fn a_failed_commit_after_reuse_keeps_the_shared_object() {
        let fixture = Fixture::new().await;
        let first = fixture.upload("first.png", b"abc").await.unwrap();
        break_version_inserts(&fixture).await;
        assert!(fixture.upload("second.png", b"abc").await.is_err());
        assert_eq!(fixture.rows().await, 2);
        assert_eq!(std::fs::read(fixture.object(ABC)).unwrap(), b"abc");
        assert_eq!(fixture.database.assets().await.unwrap(), [first]);
    }

    #[tokio::test]
    async fn a_name_with_traversal_is_display_text_only() {
        let fixture = Fixture::new().await;
        let record = fixture.upload("../../outside.png", b"abc").await.unwrap();
        assert_eq!(record.name, "../../outside.png");
        for escaped in [
            "../outside.png",
            "../../outside.png",
            "tmp/../../outside.png",
        ] {
            assert!(!fixture.temp.path.join(escaped).exists(), "{escaped}");
        }
        assert_eq!(
            listing(&fixture.temp.path),
            [crate::storage::MARKER, "store", "tmp"]
        );
    }

    #[tokio::test]
    async fn a_cancelled_ingest_publishes_nothing() {
        let fixture = Fixture::new().await;
        let outcome = fixture
            .upload_with(&fixture.objects(None), "nebula.png", b"abc", true)
            .await;
        assert!(
            matches!(outcome, Err(IngestError::Cancelled)),
            "{outcome:?}"
        );
        assert_eq!(fixture.rows().await, 0);
        assert!(!fixture.object(ABC).exists());
    }

    #[tokio::test]
    async fn an_empty_name_is_refused_before_anything_is_published() {
        let fixture = Fixture::new().await;
        let outcome = fixture.upload("", b"abc").await;
        assert!(
            matches!(outcome, Err(IngestError::EmptyName)),
            "{outcome:?}"
        );
        assert!(!fixture.object(ABC).exists());
    }
}
