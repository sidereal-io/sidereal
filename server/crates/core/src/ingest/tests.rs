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
        Self::with_isolation(None).await
    }

    /// A fixture whose Database connections start every transaction at
    /// `isolation`, when one is given, as a role or database setting could.
    async fn with_isolation(isolation: Option<&str>) -> Self {
        let db = TestDatabase::new();
        let temp = TempRoot::new();
        let options = match isolation {
            Some(level) => db
                .options
                .clone()
                .options([("default_transaction_isolation", level)]),
            None => db.options.clone(),
        };
        let (database, paired, _) =
            initialize_with_storage(options, SECONDS, SECONDS, &temp.root())
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
    async fn a_retry_after_a_failed_commit_reuses_the_kept_object() {
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

    #[test]
    fn the_content_lock_key_keeps_the_bits_of_the_first_eight_hex_digits() {
        use crate::assets::content_lock_key;
        let key = |prefix: &str| content_lock_key(&format!("{prefix}{}", &ABC[8..]));
        assert_eq!(key("00000000"), 0);
        assert_eq!(key("7fffffff"), i32::MAX);
        assert_eq!(key("80000000"), i32::MIN);
        assert_eq!(key("ffffffff"), -1);
        assert_eq!(content_lock_key(ABC), 0x6437_b3ac);
    }

    /// The advisory lock key a held upload waits on before it commits.
    const HOLD: i64 = 522;
    /// How long a test waits for another connection before it fails.
    const WAIT: Duration = Duration::from_secs(10);

    /// How many backends in this test's database wait on an advisory lock.
    async fn advisory_waiters(fixture: &Fixture) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM pg_locks l JOIN pg_stat_activity a ON a.pid = l.pid \
             WHERE a.datname = current_database() AND NOT l.granted AND l.locktype = 'advisory'",
        )
        .fetch_one(&mut fixture.conn().await)
        .await
        .unwrap()
    }

    /// Wait until `count` backends wait on an advisory lock, failing after
    /// [`WAIT`].
    async fn wait_for_waiters(fixture: &Fixture, count: i64) {
        tokio::time::timeout(WAIT, async {
            while advisory_waiters(fixture).await < count {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("waited too long for {count} advisory waiters"));
    }

    /// Two uploads of the same new bytes: A is held after its insert and
    /// before its commit, then B starts. Each must end as one new Asset and
    /// one found Asset.
    async fn concurrent_uploads_make_one_asset(isolation: Option<&str>) {
        let fixture = Arc::new(Fixture::with_isolation(isolation).await);
        let mut hold = fixture.conn().await;
        hold.execute(sqlx::AssertSqlSafe(format!(
            "SELECT pg_advisory_lock({HOLD}); \
             CREATE FUNCTION public.hold() RETURNS trigger LANGUAGE plpgsql \
             AS 'BEGIN PERFORM pg_advisory_xact_lock_shared({HOLD}); RETURN NULL; END'; \
             CREATE TRIGGER hold AFTER INSERT ON public.asset_versions \
             FOR EACH ROW EXECUTE FUNCTION public.hold()"
        )))
        .await
        .unwrap();
        let start = |name: &'static str| {
            let fixture = fixture.clone();
            tokio::spawn(async move { fixture.upload(name, b"abc").await })
        };
        let a = start("a.png");
        wait_for_waiters(&fixture, 1).await;
        let b = start("b.png");
        wait_for_waiters(&fixture, 2).await;
        assert!(!a.is_finished() && !b.is_finished());
        hold.execute(sqlx::AssertSqlSafe(format!(
            "SELECT pg_advisory_unlock({HOLD})"
        )))
        .await
        .unwrap();
        let finish = |task: tokio::task::JoinHandle<_>| async move {
            tokio::time::timeout(WAIT, task)
                .await
                .expect("waited too long for an upload")
                .unwrap()
        };
        let outcomes = [finish(a).await.unwrap(), finish(b).await.unwrap()];
        let (created, found): (Vec<_>, Vec<_>) = outcomes
            .iter()
            .partition(|outcome| matches!(outcome, Ingested::Created(_)));
        assert_eq!((created.len(), found.len()), (1, 1), "{outcomes:?}");
        let (Ingested::Created(new), Ingested::Found(existing)) = (created[0], found[0]) else {
            unreachable!()
        };
        assert_eq!(existing.id, new.id);
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT count(DISTINCT asset_id), count(*) FROM public.asset_versions WHERE content_hash = $1",
        )
        .bind(ABC)
        .fetch_one(&mut fixture.conn().await)
        .await
        .unwrap();
        assert_eq!(counts, (1, 1));
        assert_eq!(fixture.rows().await, 2);
    }

    #[tokio::test]
    async fn concurrent_uploads_of_the_same_new_bytes_make_one_asset() {
        concurrent_uploads_make_one_asset(None).await;
    }

    #[tokio::test]
    async fn concurrent_uploads_make_one_asset_under_a_repeatable_read_default() {
        concurrent_uploads_make_one_asset(Some("repeatable read")).await;
    }

    /// The content lock is per content: while an upload of `abc` is held
    /// before its commit, an upload of other bytes commits at once.
    #[tokio::test]
    async fn an_upload_of_other_bytes_does_not_wait_for_a_held_upload() {
        let fixture = Arc::new(Fixture::new().await);
        let mut hold = fixture.conn().await;
        hold.execute(sqlx::AssertSqlSafe(format!(
            "SELECT pg_advisory_lock({HOLD}); \
             CREATE FUNCTION public.hold() RETURNS trigger LANGUAGE plpgsql \
             AS 'BEGIN IF NEW.content_hash = ''{ABC}'' THEN \
             PERFORM pg_advisory_xact_lock_shared({HOLD}); END IF; RETURN NULL; END'; \
             CREATE TRIGGER hold AFTER INSERT ON public.asset_versions \
             FOR EACH ROW EXECUTE FUNCTION public.hold()"
        )))
        .await
        .unwrap();
        let a = {
            let fixture = fixture.clone();
            tokio::spawn(async move { fixture.upload("a.png", b"abc").await })
        };
        wait_for_waiters(&fixture, 1).await;
        let other = tokio::time::timeout(WAIT, fixture.upload("b.png", b"abd"))
            .await
            .expect("an upload of other bytes waited for the held upload")
            .unwrap();
        assert!(matches!(other, Ingested::Created(_)), "{other:?}");
        assert!(!a.is_finished());
        hold.execute(sqlx::AssertSqlSafe(format!(
            "SELECT pg_advisory_unlock({HOLD})"
        )))
        .await
        .unwrap();
        let held = tokio::time::timeout(WAIT, a)
            .await
            .expect("waited too long for the held upload")
            .unwrap()
            .unwrap();
        assert!(matches!(held, Ingested::Created(_)), "{held:?}");
        assert_eq!(fixture.rows().await, 4);
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

    /// Stage `bytes`, let another server's startup cleanup run on the same
    /// root, then ingest what was staged.
    async fn upload_after_cleanup(
        fixture: &Fixture,
        bytes: &[u8],
    ) -> Result<AssetRecord, IngestError> {
        let objects = fixture.objects(None);
        let mut staging = objects.begin().unwrap();
        staging.write(bytes).unwrap();
        let staged = staging.finish().unwrap();
        fixture.objects(None).remove_leftover_staging().unwrap();
        ingest(
            &fixture.database,
            &objects,
            fixture.registry.clone(),
            "test.recorder",
            "nebula.png",
            staged,
            Arc::new(AtomicBool::new(false)),
        )
        .await
    }

    #[tokio::test]
    async fn an_upload_whose_staging_file_was_cleaned_up_fails_with_a_storage_error() {
        let fixture = Fixture::new().await;
        let outcome = upload_after_cleanup(&fixture, b"abc").await;
        assert!(
            matches!(outcome, Err(IngestError::Storage(_))),
            "{outcome:?}"
        );
    }

    #[tokio::test]
    async fn an_upload_whose_staging_file_was_cleaned_up_commits_nothing() {
        let fixture = Fixture::new().await;
        let _ = upload_after_cleanup(&fixture, b"abc").await;
        assert_eq!(fixture.rows().await, 0);
    }
}

/// Stopping the process mid-ingest. The parent test runs this test binary
/// again as a child, which uploads one candidate and aborts at a chosen
/// [`Step`]. `abort` runs no destructor, so the staging file stays, as it
/// would after a crash or a kill.
mod interrupted_ingest {
    use super::*;
    use crate::storage::objects::{Cleanup, Level};
    use sqlx::ConnectOptions;
    use std::os::unix::process::{CommandExt, ExitStatusExt};

    const STEP: &str = "SIDEREAL_KILL_STEP";
    const DATABASE: &str = "SIDEREAL_KILL_DATABASE";
    const ROOT: &str = "SIDEREAL_KILL_ROOT";
    const PAIRED: &str = "SIDEREAL_KILL_PAIRED";

    /// The child: upload `abc` and abort the first time the step named in
    /// `SIDEREAL_KILL_STEP` runs. Only the parent runs it.
    #[tokio::test]
    #[ignore = "run by the interrupted_ingest tests as a child process"]
    async fn child_uploads_until_killed() {
        let Ok(target) = std::env::var(STEP) else {
            return;
        };
        let options = std::env::var(DATABASE).unwrap().parse().unwrap();
        let database = crate::database::Database::open(options, SECONDS)
            .await
            .unwrap();
        let root =
            crate::storage::StorageRoot::new(Path::new(&std::env::var(ROOT).unwrap())).unwrap();
        let paired = Uuid::try_parse(&std::env::var(PAIRED).unwrap()).unwrap();
        let hook: Hook = Arc::new(move |step| {
            if format!("{step:?}") == target {
                std::process::abort();
            }
            Ok(())
        });
        let objects = ObjectStore::open_with(&root, paired, Some(hook)).unwrap();
        let mut registry = Registry::new();
        TestPack(Arc::default()).register(&mut registry);
        let mut staging = objects.begin().unwrap();
        staging.write(b"abc").unwrap();
        let staged = staging.finish().unwrap();
        let outcome = ingest(
            &database,
            &objects,
            Arc::new(registry),
            "test.recorder",
            "nebula.png",
            staged,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
        panic!("the upload ended without reaching the step: {outcome:?}");
    }

    /// A paired root after a child was killed at `step` while it uploaded
    /// `abc`, with the Asset and Asset version row count from before.
    struct Killed {
        fixture: Fixture,
        rows_before: i64,
    }

    /// Kill an upload of `abc` at `step`. With `earlier`, an upload of the
    /// same bytes commits first, so the child reuses its object.
    async fn killed_at(step: Step, earlier: bool) -> Killed {
        let fixture = Fixture::new().await;
        if earlier {
            fixture.upload("earlier.png", b"abc").await.unwrap();
        }
        let rows_before = fixture.rows().await;
        let child = module_path!()
            .split_once("::")
            .map(|(_, path)| format!("{path}::child_uploads_until_killed"))
            .unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--ignored", "--exact", &child, "--nocapture"])
            .env(STEP, format!("{step:?}"))
            .env(DATABASE, fixture.db.options.to_url_lossy().to_string())
            .env(ROOT, &fixture.temp.path)
            .env(PAIRED, fixture.paired.to_string());
        // SAFETY: setrlimit is async-signal-safe. The abort writes no core file.
        unsafe {
            command.pre_exec(|| {
                let none = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                libc::setrlimit(libc::RLIMIT_CORE, &none);
                Ok(())
            });
        }
        let output = command.output().unwrap();
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGABRT),
            "the child was not killed at {step:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Killed {
            fixture,
            rows_before,
        }
    }

    impl Killed {
        /// Start again on the same database and root, as the next server does.
        async fn restart(&self) -> Cleanup {
            let (database, _, cleanup) = initialize_with_storage(
                self.fixture.db.options.clone(),
                SECONDS,
                SECONDS,
                &self.fixture.temp.root(),
            )
            .await
            .unwrap();
            database.close().await;
            cleanup
        }
    }

    async fn leaves_no_library_record_or_object(step: Step) {
        let killed = killed_at(step, false).await;
        assert_eq!(killed.fixture.rows().await, killed.rows_before);
        assert!(!killed.fixture.object(ABC).exists());
    }

    async fn leaves_one_staging_file_the_next_start_removes(step: Step) {
        let killed = killed_at(step, false).await;
        assert_eq!(killed.fixture.staging_files(), 1);
        assert_eq!(killed.restart().await.removed, 1);
        assert_eq!(killed.fixture.staging_files(), 0);
    }

    #[tokio::test]
    async fn killed_while_writing_staging_leaves_no_library_record_or_object() {
        leaves_no_library_record_or_object(Step::WriteStaging).await;
    }

    #[tokio::test]
    async fn killed_while_writing_staging_leaves_a_file_the_next_start_removes() {
        leaves_one_staging_file_the_next_start_removes(Step::WriteStaging).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_staging_leaves_no_library_record_or_object() {
        leaves_no_library_record_or_object(Step::SyncStaging).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_staging_leaves_a_file_the_next_start_removes() {
        leaves_one_staging_file_the_next_start_removes(Step::SyncStaging).await;
    }

    #[tokio::test]
    async fn killed_before_publication_leaves_no_library_record_or_object() {
        leaves_no_library_record_or_object(Step::Publish).await;
    }

    #[tokio::test]
    async fn killed_before_publication_leaves_a_file_the_next_start_removes() {
        leaves_one_staging_file_the_next_start_removes(Step::Publish).await;
    }

    async fn keeps_the_object_and_adds_no_record(step: Step, earlier: bool) {
        let killed = killed_at(step, earlier).await;
        assert_eq!(killed.fixture.rows().await, killed.rows_before);
        assert_eq!(std::fs::read(killed.fixture.object(ABC)).unwrap(), b"abc");
    }

    /// After a new object is published, its staging file is already gone.
    async fn leaves_tmp_empty_with_nothing_to_remove(step: Step) {
        let killed = killed_at(step, false).await;
        assert_eq!(killed.fixture.staging_files(), 0);
        assert_eq!(killed.restart().await.removed, 0);
        assert_eq!(killed.fixture.staging_files(), 0);
    }

    #[tokio::test]
    async fn killed_while_syncing_the_second_prefix_keeps_the_object_and_adds_no_record() {
        keeps_the_object_and_adds_no_record(Step::SyncDirectory(Level::SecondPrefix), false).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_the_second_prefix_leaves_tmp_empty() {
        leaves_tmp_empty_with_nothing_to_remove(Step::SyncDirectory(Level::SecondPrefix)).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_the_first_prefix_keeps_the_object_and_adds_no_record() {
        keeps_the_object_and_adds_no_record(Step::SyncDirectory(Level::FirstPrefix), false).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_the_first_prefix_leaves_tmp_empty() {
        leaves_tmp_empty_with_nothing_to_remove(Step::SyncDirectory(Level::FirstPrefix)).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_store_keeps_the_object_and_adds_no_record() {
        keeps_the_object_and_adds_no_record(Step::SyncDirectory(Level::Store), false).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_store_leaves_tmp_empty() {
        leaves_tmp_empty_with_nothing_to_remove(Step::SyncDirectory(Level::Store)).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_the_root_keeps_the_object_and_adds_no_record() {
        keeps_the_object_and_adds_no_record(Step::SyncDirectory(Level::Root), false).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_the_root_leaves_tmp_empty() {
        leaves_tmp_empty_with_nothing_to_remove(Step::SyncDirectory(Level::Root)).await;
    }

    #[tokio::test]
    async fn killed_while_syncing_a_reused_object_keeps_the_records_from_before() {
        keeps_the_object_and_adds_no_record(Step::SyncObject, true).await;
    }

    /// On reuse, the object is synced before the staging file is dropped.
    #[tokio::test]
    async fn killed_while_syncing_a_reused_object_leaves_a_file_the_next_start_removes() {
        let killed = killed_at(Step::SyncObject, true).await;
        assert_eq!(killed.fixture.staging_files(), 1);
        assert_eq!(killed.restart().await.removed, 1);
        assert_eq!(killed.fixture.staging_files(), 0);
    }
}
