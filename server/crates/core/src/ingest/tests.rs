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
    ) -> Result<Ingested, IngestError> {
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

    async fn upload(&self, name: &str, bytes: &[u8]) -> Result<Ingested, IngestError> {
        self.upload_with(&self.objects(None), name, bytes, false)
            .await
    }

    /// Upload bytes not yet in the library, and return the new Asset.
    async fn create(&self, name: &str, bytes: &[u8]) -> AssetRecord {
        match self.upload(name, bytes).await {
            Ok(Ingested::Created(record)) => record,
            other => panic!("expected a new Asset, got {other:?}"),
        }
    }

    async fn failing_at(&self, failing: Step, bytes: &[u8]) -> Result<Ingested, IngestError> {
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
        let record = fixture.create("nebula.png", b"abc").await;
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
        let record = fixture.create("nebula.png", b"abc").await;
        assert_eq!(
            fixture.database.asset(record.id).await.unwrap(),
            Some(record.clone())
        );
        assert_eq!(fixture.database.assets().await.unwrap(), [record]);
    }

    #[tokio::test]
    async fn the_object_is_published_before_the_commit() {
        let fixture = Fixture::new().await;
        let record = fixture.create("nebula.png", b"abc").await;
        assert_eq!(
            std::fs::read(fixture.object(&record.version.content_hash)).unwrap(),
            b"abc"
        );
        assert_eq!(fixture.staging_files(), 0);
    }

    #[tokio::test]
    async fn the_ids_are_fresh_uuids_not_the_name_or_hash() {
        let fixture = Fixture::new().await;
        let record = fixture.create("nebula.png", b"abc").await;
        assert_ne!(record.id, record.version.id);
        for id in [record.id, record.version.id] {
            assert_eq!(id.get_version_num(), 4);
            assert!(!ABC.contains(&id.simple().to_string()[..8]));
        }
    }

    #[tokio::test]
    async fn provenance_names_the_registered_source_that_ran() {
        let fixture = Fixture::new().await;
        let record = fixture.create("nebula.png", b"abc").await;
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
        use std::os::unix::fs::MetadataExt;
        let fixture = Fixture::new().await;
        break_version_inserts(&fixture).await;
        let outcome = fixture.upload("first.png", b"abc").await;
        assert!(
            matches!(outcome, Err(IngestError::Database(_))),
            "{outcome:?}"
        );
        assert_eq!(fixture.rows().await, 0);
        let kept = std::fs::metadata(fixture.object(ABC)).unwrap().ino();
        fixture
            .conn()
            .await
            .execute("DROP TRIGGER refuse ON public.asset_versions")
            .await
            .unwrap();
        let second = fixture.create("second.png", b"abc").await;
        assert_eq!(second.name, "second.png");
        assert_eq!(fixture.rows().await, 2);
        assert_eq!(std::fs::metadata(fixture.object(ABC)).unwrap().ino(), kept);
        assert_eq!(std::fs::read(fixture.object(ABC)).unwrap(), b"abc");
        assert_eq!(fixture.database.assets().await.unwrap(), [second]);
    }

    #[tokio::test]
    async fn the_same_bytes_under_another_name_find_the_first_asset() {
        let fixture = Fixture::new().await;
        let first = fixture.create("nebula.png", b"abc").await;
        let outcome = fixture.upload("m31-final.png", b"abc").await.unwrap();
        assert_eq!(outcome, Ingested::Found(first.clone()));
        assert_eq!(first.name, "nebula.png");
        assert_eq!(fixture.rows().await, 2);
        assert_eq!(fixture.database.assets().await.unwrap(), [first]);
    }

    #[tokio::test]
    async fn different_bytes_make_a_new_asset() {
        let fixture = Fixture::new().await;
        let first = fixture.create("nebula.png", b"abc").await;
        let second = fixture.create("nebula.jpg", b"abd").await;
        assert_ne!(first.id, second.id);
        assert_eq!(fixture.rows().await, 4);
        assert_eq!(fixture.database.assets().await.unwrap(), [first, second]);
    }

    /// One version a test inserts directly, as a database built before this
    /// change, or another Source, could hold it.
    struct Row<'a> {
        id: &'a str,
        hash: &'a str,
        source_id: &'a str,
        ingested_at: &'a str,
    }

    impl Row<'_> {
        fn new<'a>(id: &'a str, hash: &'a str, ingested_at: &'a str) -> Row<'a> {
            Row {
                id,
                hash,
                source_id: "test.recorder",
                ingested_at,
            }
        }
    }

    /// Insert Asset `id`, named `name`, with `versions`; the last one is its
    /// current version.
    async fn insert_asset(fixture: &Fixture, id: &str, name: &str, versions: &[Row<'_>]) {
        let current = versions.last().unwrap().id;
        let mut sql = format!(
            "BEGIN; INSERT INTO public.assets (id, kind, name, current_version_id) \
             VALUES ('{id}', 'image', '{name}', '{current}');"
        );
        for v in versions {
            sql += &format!(
                " INSERT INTO public.asset_versions \
                 (id, asset_id, content_hash, byte_size, media_type, source_id, source_version, ingested_at) \
                 VALUES ('{}', '{id}', '{}', 3, 'image/x-test', '{}', '1.0.0', '{}');",
                v.id, v.hash, v.source_id, v.ingested_at
            );
        }
        sql += " COMMIT;";
        fixture
            .conn()
            .await
            .execute(sqlx::AssertSqlSafe(sql))
            .await
            .unwrap();
    }

    /// Every Asset and Asset version row, as text, in id order.
    async fn snapshot(fixture: &Fixture) -> String {
        sqlx::query_scalar(
            "SELECT coalesce((SELECT string_agg(a::text, ';' ORDER BY a.id) FROM public.assets a), '') \
             || '|' || coalesce((SELECT string_agg(v::text, ';' ORDER BY v.id) FROM public.asset_versions v), '')",
        )
        .fetch_one(&mut fixture.conn().await)
        .await
        .unwrap()
    }

    fn found(outcome: Result<Ingested, IngestError>) -> AssetRecord {
        match outcome {
            Ok(Ingested::Found(record)) => record,
            other => panic!("expected a found Asset, got {other:?}"),
        }
    }

    const OLDER: &str = "a0000000-0000-4000-8000-000000000001";
    const LOW: &str = "00000000-0000-4000-8000-000000000001";
    const HIGH: &str = "ffffffff-0000-4000-8000-000000000001";
    const OTHER: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[tokio::test]
    async fn an_upload_finds_the_asset_whose_version_was_ingested_first() {
        let fixture = Fixture::new().await;
        fixture.create("nebula.png", b"abc").await;
        insert_asset(
            &fixture,
            OLDER,
            "older.png",
            &[Row::new(
                "a0000000-0000-4000-8000-0000000000a1",
                ABC,
                "2000-01-01T00:00:00Z",
            )],
        )
        .await;
        let before = snapshot(&fixture).await;
        let record = found(fixture.upload("again.png", b"abc").await);
        assert_eq!(
            (record.id.to_string().as_str(), record.name.as_str()),
            (OLDER, "older.png")
        );
        assert_eq!(snapshot(&fixture).await, before);
    }

    #[tokio::test]
    async fn among_versions_ingested_together_an_upload_finds_the_lowest_asset_id() {
        let fixture = Fixture::new().await;
        let at = "2000-01-01T00:00:00Z";
        insert_asset(
            &fixture,
            HIGH,
            "high.png",
            &[Row::new("ffffffff-0000-4000-8000-0000000000a1", ABC, at)],
        )
        .await;
        insert_asset(
            &fixture,
            LOW,
            "low.png",
            &[Row::new("00000000-0000-4000-8000-0000000000a1", ABC, at)],
        )
        .await;
        let before = snapshot(&fixture).await;
        let record = found(fixture.upload("again.png", b"abc").await);
        assert_eq!(record.id.to_string(), LOW);
        assert_eq!(snapshot(&fixture).await, before);
    }

    #[tokio::test]
    async fn an_upload_finds_an_asset_whose_matching_version_is_not_current() {
        let fixture = Fixture::new().await;
        let current = "a0000000-0000-4000-8000-0000000000a2";
        insert_asset(
            &fixture,
            OLDER,
            "older.png",
            &[
                Row::new(
                    "a0000000-0000-4000-8000-0000000000a1",
                    ABC,
                    "2000-01-01T00:00:00Z",
                ),
                Row::new(current, OTHER, "2000-01-02T00:00:00Z"),
            ],
        )
        .await;
        let before = snapshot(&fixture).await;
        let record = found(fixture.upload("again.png", b"abc").await);
        assert_eq!(record.id.to_string(), OLDER);
        assert_eq!(record.current_version_id.to_string(), current);
        assert_eq!(record.version.content_hash, OTHER);
        assert_eq!(snapshot(&fixture).await, before);
    }

    #[tokio::test]
    async fn bytes_another_source_brought_in_are_found_and_keep_their_provenance() {
        let fixture = Fixture::new().await;
        insert_asset(
            &fixture,
            OLDER,
            "older.png",
            &[Row {
                source_id: "test.other",
                ..Row::new(
                    "a0000000-0000-4000-8000-0000000000a1",
                    ABC,
                    "2000-01-01T00:00:00Z",
                )
            }],
        )
        .await;
        let before = snapshot(&fixture).await;
        let record = found(fixture.upload("again.png", b"abc").await);
        assert_eq!(*fixture.seen.lock().unwrap(), b"abc");
        assert_eq!(record.id.to_string(), OLDER);
        assert_eq!(
            (
                record.version.source_id.as_str(),
                record.version.source_version.as_str()
            ),
            ("test.other", "1.0.0")
        );
        assert_eq!(snapshot(&fixture).await, before);
    }

    #[tokio::test]
    async fn refused_bytes_already_in_the_library_are_still_refused() {
        let fixture = Fixture::new().await;
        let rejected = blake3::hash(b"rejected").to_hex().to_string();
        insert_asset(
            &fixture,
            OLDER,
            "older.png",
            &[Row::new(
                "a0000000-0000-4000-8000-0000000000a1",
                &rejected,
                "2000-01-01T00:00:00Z",
            )],
        )
        .await;
        let before = snapshot(&fixture).await;
        let outcome = fixture.upload("again.png", b"rejected").await;
        assert!(
            matches!(outcome, Err(IngestError::Refused(_))),
            "{outcome:?}"
        );
        assert_eq!(snapshot(&fixture).await, before);
    }

    #[tokio::test]
    async fn a_name_with_traversal_is_display_text_only() {
        let fixture = Fixture::new().await;
        let record = fixture.create("../../outside.png", b"abc").await;
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
