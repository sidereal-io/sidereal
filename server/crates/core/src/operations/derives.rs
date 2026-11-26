//! An Operator that makes previews: core checks the proposed preview, stages
//! and publishes its bytes, and commits a derived Asset, its version and one
//! lineage row with the run (ADR-018).
use super::fixture::*;
use super::*;
use crate::{
    assets::AssetRecord,
    storage::objects::{Hook, Step},
};
use sidereal_plugin_abi::{AssetContext, AssetProposal, ContextError, Operator, OperatorError};
use sqlx::{Executor, Row};
use std::sync::atomic::AtomicUsize;

type Script = fn(&mut AssetContext<'_>) -> Result<(), OperatorError>;

/// `test.op`, which makes previews and writes `test.size`.
fn previewing(script: Script) -> Box<dyn Operator> {
    Box::new(Scripted {
        id: "test.op",
        version: "1.2.3",
        writes: &["test.size"],
        accepts: &["image/x-test"],
        makes_previews: true,
        script,
    })
}

async fn fixture_with(operator: fn(Script) -> Box<dyn Operator>, script: Script) -> Fixture {
    Fixture::new(move |r| {
        r.declare_facet(count("test.size"));
        r.register_operator(operator(script));
    })
    .await
}

const PREVIEW: &[u8] = b"0123456789";

fn jpeg() -> AssetProposal {
    AssetProposal::new("image", "image/jpeg")
}

/// Propose a JPEG preview and write `PREVIEW` in two writes.
fn writes_a_jpeg(context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
    context.propose_preview(jpeg())?;
    context.write_preview(&PREVIEW[..4])?;
    context.write_preview(&PREVIEW[4..])?;
    Ok(())
}

/// One derived Asset, version and lineage row, as stored.
#[derive(Debug)]
struct Derived {
    asset: Uuid,
    kind: String,
    name: String,
    current: Uuid,
    version: Uuid,
    hash: String,
    size: i64,
    media_type: String,
    source_id: String,
    source_version: String,
    input: Uuid,
    role: String,
    run: Uuid,
}

impl Fixture {
    /// Every derived Asset with its version and lineage row.
    async fn derived(&self) -> Vec<Derived> {
        sqlx::query(
            "SELECT a.id, a.kind, a.name, a.current_version_id, v.id, v.content_hash, \
             v.byte_size, v.media_type, v.source_id, v.source_version, \
             l.input_version_id, l.role, l.run_id \
             FROM public.assets a JOIN public.asset_versions v ON v.asset_id = a.id \
             JOIN public.asset_lineage l ON l.output_version_id = v.id \
             WHERE a.origin = 'derived' ORDER BY a.id",
        )
        .fetch_all(&mut self.conn().await)
        .await
        .unwrap()
        .iter()
        .map(|row| Derived {
            asset: row.get(0),
            kind: row.get(1),
            name: row.get(2),
            current: row.get(3),
            version: row.get(4),
            hash: row.get(5),
            size: row.get(6),
            media_type: row.get(7),
            source_id: row.get(8),
            source_version: row.get(9),
            input: row.get(10),
            role: row.get(11),
            run: row.get(12),
        })
        .collect()
    }

    /// No derived Asset, no Asset version beyond the original's and no
    /// lineage row exists, and the storage folder's `tmp` is empty.
    async fn assert_no_preview(&self) {
        let counts: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM public.assets), \
             (SELECT count(*) FROM public.assets WHERE origin = 'derived'), \
             (SELECT count(*) FROM public.asset_versions), \
             (SELECT count(*) FROM public.asset_lineage)",
        )
        .fetch_one(&mut self.conn().await)
        .await
        .unwrap();
        assert_eq!(counts, (1, 0, 1, 0));
        assert_eq!(self.staging_files(), 0);
    }

    /// How many files the storage folder's `tmp` holds.
    fn staging_files(&self) -> usize {
        std::fs::read_dir(self.temp.path.join("tmp"))
            .map(|entries| entries.count())
            .unwrap_or(0)
    }

    /// Run `test.op` over `version` through an object store whose every
    /// filesystem step first calls `hook`.
    async fn run_hooked(
        &self,
        version: &AssetRecord,
        hook: Hook,
        cancelled: Arc<AtomicBool>,
    ) -> RunOutcome {
        let objects = ObjectStore::open_with(&self.temp.root(), self.paired, Some(hook)).unwrap();
        run_operator(
            &self.database,
            &objects,
            self.registry.clone(),
            RunRequest {
                operator_id: "test.op".into(),
                asset_version_id: version.version.id,
                actor: self.nova,
            },
            cancelled,
        )
        .await
        .unwrap()
    }
}

#[tokio::test]
async fn a_test_operator_proposes_a_png_preview_and_it_commits() {
    let fixture = fixture_with(previewing, |context| {
        context.propose_preview(AssetProposal::new("image", "image/png"))?;
        context.write_preview(PREVIEW)?;
        Ok(())
    })
    .await;
    let version = fixture.version(b"png bytes").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    let derived = fixture.derived().await;
    assert_eq!(derived.len(), 1, "{derived:?}");
    let preview = &derived[0];
    assert_eq!(preview.kind, "image");
    assert_eq!(preview.media_type, "image/png");
    assert_eq!(preview.size, 10);
    assert_eq!(
        std::fs::read(fixture.object(&preview.hash)).unwrap(),
        PREVIEW
    );
    assert_eq!(fixture.staging_files(), 0);
}

#[tokio::test]
async fn the_recorded_preview_is_a_derived_asset_linked_to_its_input() {
    let fixture = fixture_with(previewing, writes_a_jpeg).await;
    let version = fixture.version(b"png bytes").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    let runs = fixture.runs().await;
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "succeeded");
    let derived = fixture.derived().await;
    assert_eq!(derived.len(), 1, "{derived:?}");
    let preview = &derived[0];
    assert_ne!(preview.asset, version.id);
    assert_eq!(preview.kind, "image");
    assert_eq!(preview.name, "nebula.png");
    assert_eq!(preview.current, preview.version);
    assert_eq!(preview.hash, blake3::hash(PREVIEW).to_hex().to_string());
    assert_eq!(preview.size, PREVIEW.len() as i64);
    assert_eq!(preview.media_type, "image/jpeg");
    assert_eq!(
        (preview.source_id.as_str(), preview.source_version.as_str()),
        ("test.op", "1.2.3")
    );
    assert_eq!(preview.input, version.version.id);
    assert_eq!(preview.role, "preview");
    assert_eq!(preview.run, runs[0].id);
}

/// The one run failed with `reason`, and the run left no preview.
async fn assert_refused(fixture: &Fixture, outcome: RunOutcome, reason: &str) {
    assert_eq!(outcome, RunOutcome::Failed(reason.into()));
    assert_eq!(fixture.failed_reason().await, reason);
    fixture.assert_no_preview().await;
    assert!(fixture.values().await.is_empty());
}

#[tokio::test]
async fn an_operator_that_did_not_declare_previews_is_refused() {
    fn plain(script: Script) -> Box<dyn Operator> {
        Box::new(Scripted {
            id: "test.op",
            version: "1.2.3",
            writes: &["test.size"],
            accepts: &["image/x-test"],
            makes_previews: false,
            script,
        })
    }
    let fixture = fixture_with(plain, writes_a_jpeg).await;
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture.run(&version).await.unwrap();
    assert_refused(&fixture, outcome, "operator test.op does not make previews").await;
}

#[tokio::test]
async fn a_second_preview_in_one_run_is_refused() {
    let fixture = fixture_with(previewing, |context| {
        writes_a_jpeg(context)?;
        let _ = context.propose_preview(jpeg());
        Ok(())
    })
    .await;
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture.run(&version).await.unwrap();
    assert_refused(&fixture, outcome, "proposed more than one preview").await;
}

#[tokio::test]
async fn a_preview_with_no_bytes_is_refused() {
    let fixture = fixture_with(previewing, |context| {
        context.propose_preview(jpeg())?;
        Ok(())
    })
    .await;
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture.run(&version).await.unwrap();
    assert_refused(&fixture, outcome, "the preview is empty").await;
}

#[tokio::test]
async fn a_preview_a_browser_may_not_show_is_refused() {
    let fixture = fixture_with(previewing, |context| {
        context.propose_preview(AssetProposal::new("image", "image/tiff"))?;
        context.write_preview(PREVIEW)?;
        Ok(())
    })
    .await;
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture.run(&version).await.unwrap();
    assert_refused(&fixture, outcome, "a preview cannot be image/tiff").await;
}

#[tokio::test]
async fn a_preview_over_100_mib_is_refused_and_leaves_no_staging_file() {
    let fixture = fixture_with(previewing, |context| {
        context.propose_preview(jpeg())?;
        let chunk = vec![7u8; 1024 * 1024];
        for _ in 0..100 {
            context.write_preview(&chunk)?;
        }
        context.write_preview(&[7])?;
        Ok(())
    })
    .await;
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture.run(&version).await.unwrap();
    assert_refused(&fixture, outcome, "the preview is larger than 100 MiB").await;
}

#[tokio::test]
async fn a_refused_facet_stops_the_preview_too() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.declare_facet(count("test.width"));
        r.register_operator(previewing(|context| {
            writes_a_jpeg(context)?;
            context
                .propose_facet("test.width", FacetValue::Integer(1))
                .map_err(|_| OperatorError::Failed("proposal".into()))
        }));
    })
    .await;
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture.run(&version).await.unwrap();
    assert_refused(
        &fixture,
        outcome,
        "facet test.width is not granted to test.op",
    )
    .await;
}

#[tokio::test]
async fn a_refused_commit_leaves_no_preview() {
    let fixture = fixture_with(previewing, writes_a_jpeg).await;
    let version = fixture.version(b"png bytes").await;
    fixture
        .conn()
        .await
        .execute(
            "CREATE FUNCTION public.refuse() RETURNS trigger LANGUAGE plpgsql AS \
             $$ BEGIN RAISE EXCEPTION 'injected'; END $$; \
             CREATE CONSTRAINT TRIGGER refuse AFTER INSERT ON public.asset_lineage \
             DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION public.refuse();",
        )
        .await
        .unwrap();
    let outcome = fixture.run(&version).await.unwrap();
    assert_ne!(outcome, RunOutcome::Succeeded);
    let runs = fixture.runs().await;
    assert_eq!(runs.len(), 1);
    assert_ne!(runs[0].status, "succeeded");
    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM public.assets WHERE origin = 'derived'), \
         (SELECT count(*) FROM public.asset_versions), \
         (SELECT count(*) FROM public.asset_lineage)",
    )
    .fetch_one(&mut fixture.conn().await)
    .await
    .unwrap();
    assert_eq!(counts, (0, 1, 0));
}

/// A hook that fails the `nth` staging write, counting from 1.
fn failing_write(nth: usize) -> Hook {
    let writes = Arc::new(AtomicUsize::new(0));
    Arc::new(move |step| {
        if step == Step::WriteStaging && writes.fetch_add(1, Ordering::SeqCst) + 1 == nth {
            Err(std::io::Error::other("the disk is full"))
        } else {
            Ok(())
        }
    })
}

/// No published object holds `PREVIEW`.
fn assert_unpublished(fixture: &Fixture) {
    let hash = blake3::hash(PREVIEW).to_hex().to_string();
    assert!(!fixture.object(&hash).exists());
}

#[tokio::test]
async fn an_operator_that_ignores_a_storage_write_failure_publishes_nothing() {
    let fixture = fixture_with(previewing, |context| {
        context.propose_preview(jpeg())?;
        context.write_preview(&PREVIEW[..4])?;
        assert_eq!(
            context.write_preview(&PREVIEW[4..]),
            Err(ContextError::Write)
        );
        Ok(())
    })
    .await;
    // The original's ingest wrote once; the preview's second write fails.
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture
        .run_hooked(&version, failing_write(2), Arc::default())
        .await;
    assert_refused(&fixture, outcome, "the preview could not be stored").await;
    assert_unpublished(&fixture);
}

#[tokio::test]
async fn an_operator_stopped_by_a_write_failure_records_the_staging_reason() {
    let fixture = fixture_with(previewing, writes_a_jpeg).await;
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture
        .run_hooked(&version, failing_write(1), Arc::default())
        .await;
    assert_refused(&fixture, outcome, "the preview could not be stored").await;
    assert_unpublished(&fixture);
}

#[tokio::test]
async fn an_operator_that_fails_for_its_own_reason_after_a_write_failure_keeps_it() {
    let fixture = fixture_with(previewing, |context| {
        context.propose_preview(jpeg())?;
        assert_eq!(context.write_preview(PREVIEW), Err(ContextError::Write));
        Err(OperatorError::Failed("stopped early".into()))
    })
    .await;
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture
        .run_hooked(&version, failing_write(1), Arc::default())
        .await;
    assert_refused(&fixture, outcome, "stopped early").await;
    assert_unpublished(&fixture);
}

#[tokio::test]
async fn a_failed_publication_records_that_the_preview_could_not_be_stored() {
    let fixture = fixture_with(previewing, writes_a_jpeg).await;
    let version = fixture.version(b"png bytes").await;
    let hook: Hook = Arc::new(|step| {
        if step == Step::Publish {
            Err(std::io::Error::other("the disk is full"))
        } else {
            Ok(())
        }
    });
    let outcome = fixture.run_hooked(&version, hook, Arc::default()).await;
    assert_refused(&fixture, outcome, "the preview could not be stored").await;
    assert_unpublished(&fixture);
}

#[tokio::test]
async fn a_run_cancelled_while_its_preview_is_published_records_no_preview() {
    let fixture = fixture_with(previewing, writes_a_jpeg).await;
    let version = fixture.version(b"png bytes").await;
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let hook: Hook = Arc::new(move |step| {
        if step == Step::Publish {
            flag.store(true, Ordering::SeqCst);
        }
        Ok(())
    });
    let outcome = fixture.run_hooked(&version, hook, cancelled).await;
    assert_refused(&fixture, outcome, "the run was cancelled").await;
}

/// Writes a JPEG preview, then the host cancels the run.
struct CancelledAfterWriting(Arc<AtomicBool>);
impl Operator for CancelledAfterWriting {
    fn id(&self) -> &str {
        "test.op"
    }
    fn version(&self) -> &str {
        "1.2.3"
    }
    fn writes(&self) -> &[&str] {
        &[]
    }
    fn accepts(&self) -> &[&str] {
        &["image/x-test"]
    }
    fn makes_previews(&self) -> bool {
        true
    }
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        writes_a_jpeg(context)?;
        self.0.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn a_run_cancelled_after_it_proposed_a_preview_records_no_preview() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let fixture =
        Fixture::new(move |r| r.register_operator(Box::new(CancelledAfterWriting(flag.clone()))))
            .await;
    let version = fixture.version(b"png bytes").await;
    let outcome = fixture
        .run_with(fixture.registry.clone(), "test.op", &version, cancelled)
        .await
        .unwrap();
    assert_refused(&fixture, outcome, "the run was cancelled").await;
    assert_unpublished(&fixture);
}
