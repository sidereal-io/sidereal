//! Core refuses a run's proposals, records the run `failed` with a reason
//! that names the facet, and stores no value from it. An Operator run that
//! proposes an Asset is also recorded `failed` and stores no value, and adds
//! no Asset or Asset version.
use super::fixture::*;
use super::*;
use sidereal_plugin_abi::{
    AssetContext, AssetProposal, FacetSchema, FacetScope, FacetValue, Operator, OperatorError,
};
use sqlx::Executor;
use std::sync::atomic::Ordering;

fn propose(context: &mut AssetContext<'_>, name: &str, value: i64) -> Result<(), OperatorError> {
    context
        .propose_facet(name, FacetValue::Integer(value))
        .map_err(|_| OperatorError::Failed("proposal".into()))
}

/// Run `test.op` once over a new version and answer the failed run's reason,
/// after checking that no value was stored.
async fn refused(fixture: &Fixture) -> String {
    let version = fixture.version(b"abc").await;
    let outcome = fixture.run(&version).await.unwrap();
    let reason = fixture.failed_reason().await;
    assert_eq!(outcome, RunOutcome::Failed(reason.clone()));
    assert!(fixture.values().await.is_empty());
    reason
}

#[tokio::test]
async fn an_unknown_facet_is_refused() {
    let fixture = Fixture::new(|r| {
        r.register_operator(op(&["test.missing"], |c| propose(c, "test.missing", 1)))
    })
    .await;
    let reason = refused(&fixture).await;
    assert!(reason.contains("test.missing"), "{reason}");
}

#[tokio::test]
async fn a_facet_not_granted_is_refused() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.declare_facet(count("test.width"));
        r.register_operator(op(&["test.size"], |c| propose(c, "test.width", 1)));
    })
    .await;
    let reason = refused(&fixture).await;
    assert!(reason.contains("test.width"), "{reason}");
}

/// Lists only `test.size` when it registers, and both facets afterwards.
struct Widening(Arc<AtomicBool>);
impl Operator for Widening {
    fn id(&self) -> &str {
        "test.op"
    }
    fn version(&self) -> &str {
        "1"
    }
    fn writes(&self) -> &[&str] {
        if self.0.load(Ordering::SeqCst) {
            &["test.size", "test.width"]
        } else {
            &["test.size"]
        }
    }
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        propose(context, "test.width", 1)
    }
}

#[tokio::test]
async fn a_grant_added_after_registration_is_refused() {
    let widened = Arc::new(AtomicBool::new(false));
    let flag = widened.clone();
    let fixture = Fixture::new(move |r| {
        r.declare_facet(count("test.size"));
        r.declare_facet(count("test.width"));
        r.register_operator(Box::new(Widening(flag.clone())));
    })
    .await;
    widened.store(true, Ordering::SeqCst);
    let reason = refused(&fixture).await;
    assert!(reason.contains("test.width"), "{reason}");
}

#[tokio::test]
async fn a_negative_count_is_refused() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(op(&["test.size"], |c| propose(c, "test.size", -1)));
    })
    .await;
    let reason = refused(&fixture).await;
    assert!(reason.contains("test.size"), "{reason}");
}

#[tokio::test]
async fn a_count_above_the_largest_is_refused() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(op(&["test.size"], |c| {
            propose(c, "test.size", 9_007_199_254_740_992)
        }));
    })
    .await;
    let reason = refused(&fixture).await;
    assert!(reason.contains("test.size"), "{reason}");
}

#[tokio::test]
async fn one_refused_proposal_among_valid_ones_stores_none() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.declare_facet(count("test.width"));
        r.register_operator(op(&["test.size"], |c| {
            propose(c, "test.size", 3)?;
            propose(c, "test.width", 4)
        }));
    })
    .await;
    refused(&fixture).await;
}

fn writes_size(unit: &'static str, label: &'static str, scope: FacetScope) -> Arc<Registry> {
    registry(move |r| {
        r.declare_facet(FacetSchema {
            unit: unit.into(),
            label: label.into(),
            scope,
            ..count("test.size")
        });
        r.register_operator(op(&["test.size"], |c| propose(c, "test.size", 5)));
    })
}

#[tokio::test]
async fn a_facet_changed_without_a_new_version_is_refused_and_the_value_keeps_its_unit() {
    let fixture = Fixture::new(|_| {}).await;
    let version = fixture.version(b"abc").await;
    let before = writes_size("bytes", "Size", FacetScope::AssetVersion);
    assert_eq!(
        fixture
            .run_with(before, "test.op", &version, Arc::default())
            .await
            .unwrap(),
        RunOutcome::Succeeded
    );
    for changed in [
        writes_size("KiB", "Size", FacetScope::AssetVersion),
        writes_size("bytes", "Bytes", FacetScope::AssetVersion),
        writes_size("bytes", "Size", FacetScope::Asset),
    ] {
        let outcome = fixture
            .run_with(changed, "test.op", &version, Arc::default())
            .await
            .unwrap();
        let RunOutcome::Failed(reason) = outcome else {
            panic!("{outcome:?}")
        };
        assert!(
            reason.contains("test.size") && reason.contains("version 1"),
            "{reason}"
        );
    }
    let values = fixture.values().await;
    assert_eq!(values.len(), 1);
    assert_eq!(
        (values[0].unit.as_str(), values[0].label.as_str()),
        ("bytes", "Size")
    );
    let failed: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.operation_runs WHERE status = 'failed'")
            .fetch_one(&mut fixture.conn().await)
            .await
            .unwrap();
    assert_eq!(failed, 3);
}

#[tokio::test]
async fn a_kept_schema_with_another_type_or_owner_is_refused() {
    for kept in [
        "('test.size', 1, 'test', 'Size', 'text', 'bytes', 'asset_version')",
        "('test.size', 1, 'elsewhere', 'Size', 'count', 'bytes', 'asset_version')",
    ] {
        let fixture = Fixture::new(|r| {
            r.declare_facet(count("test.size"));
            r.register_operator(op(&["test.size"], |c| propose(c, "test.size", 5)));
        })
        .await;
        fixture
            .conn()
            .await
            .execute(sqlx::AssertSqlSafe(format!(
                "INSERT INTO public.facet_schemas \
                 (name, version, owner_pack, label, value_type, unit, scope) VALUES {kept}"
            )))
            .await
            .unwrap();
        let reason = refused(&fixture).await;
        assert!(
            reason.contains("test.size") && reason.contains("version 1"),
            "{kept}: {reason}"
        );
    }
}

/// The counts of Assets and Asset versions in the library.
async fn library(fixture: &Fixture) -> (i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM public.assets), (SELECT count(*) FROM public.asset_versions)",
    )
    .fetch_one(&mut fixture.conn().await)
    .await
    .unwrap()
}

/// Like `refused`, after also checking that the run added no Asset or
/// Asset version.
async fn refused_without_an_asset(fixture: &Fixture) -> String {
    let version = fixture.version(b"abc").await;
    let before = library(fixture).await;
    let outcome = fixture.run(&version).await.unwrap();
    assert_eq!(library(fixture).await, before);
    let reason = fixture.failed_reason().await;
    assert_eq!(outcome, RunOutcome::Failed(reason.clone()));
    assert!(fixture.values().await.is_empty());
    reason
}

fn an_image() -> AssetProposal {
    AssetProposal::new("image", "image/x-test")
}

#[tokio::test]
async fn an_operator_that_proposes_an_asset_and_succeeds_is_refused() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(op(&["test.size"], |c| {
            propose(c, "test.size", 3)?;
            let _ignored = c.propose_asset(an_image());
            Ok(())
        }));
    })
    .await;
    let reason = refused_without_an_asset(&fixture).await;
    assert!(reason.contains("proposed an asset"), "{reason}");
}

#[tokio::test]
async fn an_operator_that_proposes_an_asset_and_stops_keeps_its_own_reason() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(op(&["test.size"], |c| {
            propose(c, "test.size", 3)?;
            c.propose_asset(an_image())
                .map_err(|_| OperatorError::Failed("could not make a preview".into()))
        }));
    })
    .await;
    assert_eq!(
        refused_without_an_asset(&fixture).await,
        "could not make a preview"
    );
}
