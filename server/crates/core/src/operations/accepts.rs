//! Core runs an Operator only over an original's version whose media type it
//! accepts, and records nothing for any other version (ADR-019).
use super::fixture::*;
use super::*;
use crate::assets::AssetRecord;
use sidereal_plugin_abi::{AssetContext, Operator, OperatorError};
use sqlx::Connection;
use std::sync::atomic::AtomicUsize;

/// An Operator `test.op` that accepts `accepts` and writes `test.size`.
fn accepting(
    accepts: &'static [&'static str],
    script: fn(&mut AssetContext<'_>) -> Result<(), OperatorError>,
) -> Box<dyn Operator> {
    Box::new(Scripted {
        id: "test.op",
        version: "1.2.3",
        writes: &["test.size"],
        accepts,
        makes_previews: false,
        script,
    })
}

fn size(context: &mut AssetContext<'_>, value: i64) -> Result<(), OperatorError> {
    context
        .propose_facet("test.size", FacetValue::Integer(value))
        .map_err(|_| OperatorError::Failed("proposal".into()))
}

/// No run row and no facet value exists.
async fn assert_nothing_recorded(fixture: &Fixture) {
    assert!(fixture.runs().await.is_empty());
    assert!(fixture.values().await.is_empty());
}

static RAN: AtomicUsize = AtomicUsize::new(0);

#[tokio::test]
async fn a_media_type_the_operator_does_not_accept_records_no_run() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(accepting(&["image/png"], |context| {
            RAN.fetch_add(1, Ordering::SeqCst);
            size(context, 1)
        }));
    })
    .await;
    let version = fixture.version_of(b"jpeg bytes", "image/jpeg").await;
    assert_eq!(
        fixture.run(&version).await.unwrap(),
        RunOutcome::NotAccepted
    );
    assert_eq!(RAN.load(Ordering::SeqCst), 0, "the Operator ran");
    assert_nothing_recorded(&fixture).await;
}

/// Insert a derived Asset of kind `image` whose one version, with media
/// type `image/jpeg`, is the preview of `input`, made by a succeeded run.
/// Its id.
async fn insert_preview(fixture: &Fixture, input: &AssetRecord) -> Uuid {
    let (asset, version, run) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut conn = fixture.conn().await;
    let mut tx = conn.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO public.operation_runs \
         (id, operator_id, operator_version, asset_version_id, actor_user_id, status, started_at, ended_at) \
         VALUES ($1, 'test.preview', '1', $2, $3, 'succeeded', now(), now())",
    )
    .bind(run)
    .bind(input.version.id)
    .bind(fixture.nova)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO public.assets (id, kind, name, current_version_id, origin) \
         VALUES ($1, 'image', 'nebula.png', $2, 'derived')",
    )
    .bind(asset)
    .bind(version)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO public.asset_versions (id, asset_id, content_hash, byte_size, media_type, \
         source_id, source_version, ingested_at) \
         VALUES ($1, $2, $3, 3, 'image/jpeg', 'test.preview', '1', now())",
    )
    .bind(version)
    .bind(asset)
    .bind("e".repeat(64))
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO public.asset_lineage (output_version_id, input_version_id, role, run_id) \
         VALUES ($1, $2, 'preview', $3)",
    )
    .bind(version)
    .bind(input.version.id)
    .bind(run)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    version
}

#[tokio::test]
async fn a_previews_version_is_accepted_by_no_operator() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(accepting(&["image/jpeg"], |context| size(context, 1)));
    })
    .await;
    let original = fixture.version_of(b"jpeg bytes", "image/jpeg").await;
    let preview = insert_preview(&fixture, &original).await;
    assert_eq!(
        fixture
            .run_version(fixture.registry.clone(), "test.op", preview, Arc::default())
            .await
            .unwrap(),
        RunOutcome::NotAccepted
    );
    let runs = fixture.runs().await;
    assert!(
        runs.iter().all(|run| run.asset_version_id != preview),
        "{runs:?}"
    );
    assert!(fixture.values().await.is_empty());
    let previews_of_the_preview: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.asset_lineage WHERE input_version_id = $1")
            .bind(preview)
            .fetch_one(&mut fixture.conn().await)
            .await
            .unwrap();
    assert_eq!(previews_of_the_preview, 0);
}

/// Accepts only `image/png` when it registers, and `image/gif` too
/// afterwards.
struct Widening(Arc<AtomicBool>);
impl Operator for Widening {
    fn id(&self) -> &str {
        "test.op"
    }
    fn version(&self) -> &str {
        "1"
    }
    fn writes(&self) -> &[&str] {
        &["test.size"]
    }
    fn accepts(&self) -> &[&str] {
        if self.0.load(Ordering::SeqCst) {
            &["image/png", "image/gif"]
        } else {
            &["image/png"]
        }
    }
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        size(context, 1)
    }
}

#[tokio::test]
async fn a_media_type_accepted_only_after_registration_records_no_run() {
    let widened = Arc::new(AtomicBool::new(false));
    let flag = widened.clone();
    let fixture = Fixture::new(move |r| {
        r.declare_facet(count("test.size"));
        r.register_operator(Box::new(Widening(flag.clone())));
    })
    .await;
    widened.store(true, Ordering::SeqCst);
    let version = fixture.version_of(b"gif bytes", "image/gif").await;
    assert_eq!(
        fixture.run(&version).await.unwrap(),
        RunOutcome::NotAccepted
    );
    assert_nothing_recorded(&fixture).await;
}

#[tokio::test]
async fn the_operator_reads_the_versions_media_type() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(accepting(&["image/png"], |context| {
            let len = context.media_type().map_or(0, str::len) as i64;
            size(context, len)
        }));
    })
    .await;
    let version = fixture.version_of(b"png bytes", "image/png").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    let values = fixture.values().await;
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].value, serde_json::json!(9));
    assert_eq!(values[0].asset_version_id, Some(version.version.id));
}
