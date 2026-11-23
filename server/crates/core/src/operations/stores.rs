//! A run whose proposals all pass stores each value with its provenance.
use super::fixture::*;
use super::*;
use sidereal_plugin_abi::{FacetSchema, FacetScope, FacetValue, OperatorError, MAX_COUNT};

#[tokio::test]
async fn a_test_operator_stores_the_first_byte_it_read() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.first-byte"));
        r.register_operator(op(&["test.first-byte"], |context| {
            let mut first = [0; 1];
            context
                .read_candidate(0, &mut first)
                .map_err(|_| OperatorError::Failed("read".into()))?;
            context
                .propose_facet("test.first-byte", FacetValue::Integer(first[0].into()))
                .map_err(|_| OperatorError::Failed("proposal".into()))
        }));
    })
    .await;
    let version = fixture.version(b"*bytes").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    let runs = fixture.runs().await;
    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    assert_eq!(run.operator_id, "test.op");
    assert_eq!(run.operator_version, "1.2.3");
    assert_eq!(run.asset_version_id, version.version.id);
    assert_eq!(run.actor, fixture.nova);
    assert_eq!(run.status, "succeeded");
    assert_eq!(run.reason, None);
    assert_eq!(run.ended_after_start, Some(true));
    let values = fixture.values().await;
    assert_eq!(values.len(), 1);
    let value = &values[0];
    assert_eq!(value.facet, "test.first-byte");
    assert_eq!(value.value, serde_json::json!(42));
    assert_eq!(value.schema_version, 1);
    assert_eq!(value.asset_version_id, Some(version.version.id));
    assert_eq!(value.asset_id, None);
    assert_eq!(
        (value.producer_id.as_str(), value.producer_version.as_str()),
        ("test.op", "1.2.3")
    );
    assert_eq!(value.run_id, run.id);
    assert!(value.written);
    assert_eq!(
        (value.label.as_str(), value.unit.as_str()),
        ("Size", "bytes")
    );
}

#[tokio::test]
async fn the_schema_version_row_is_kept_with_its_owner() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(FacetSchema {
            version: 3,
            ..count("test.size")
        });
        r.register_operator(op(&["test.size"], |context| {
            context
                .propose_facet("test.size", FacetValue::Integer(1))
                .map_err(|_| OperatorError::Failed("p".into()))
        }));
    })
    .await;
    let version = fixture.version(b"abc").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    let row: (String, i32, String, String, String, String, String) = sqlx::query_as(
        "SELECT name, version, owner_pack, label, value_type, unit, scope FROM public.facet_schemas",
    )
    .fetch_one(&mut fixture.conn().await)
    .await
    .unwrap();
    assert_eq!(
        row,
        (
            "test.size".into(),
            3,
            "test".into(),
            "Size".into(),
            "count".into(),
            "bytes".into(),
            "asset_version".into()
        )
    );
    assert_eq!(fixture.values().await[0].schema_version, 3);
}

#[tokio::test]
async fn an_asset_value_belongs_to_the_asset_that_owns_the_version() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(FacetSchema {
            scope: FacetScope::Asset,
            ..count("test.whole")
        });
        r.register_operator(op(&["test.whole"], |context| {
            context
                .propose_facet("test.whole", FacetValue::Integer(7))
                .map_err(|_| OperatorError::Failed("p".into()))
        }));
    })
    .await;
    let version = fixture.version(b"abc").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    let values = fixture.values().await;
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].asset_id, Some(version.id));
    assert_eq!(values[0].asset_version_id, None);
}

#[tokio::test]
async fn the_largest_count_is_stored() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(op(&["test.size"], |context| {
            context
                .propose_facet("test.size", FacetValue::Integer(MAX_COUNT))
                .map_err(|_| OperatorError::Failed("p".into()))
        }));
    })
    .await;
    let version = fixture.version(b"abc").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    assert_eq!(
        fixture.values().await[0].value,
        serde_json::json!(9_007_199_254_740_991_i64)
    );
}

#[tokio::test]
async fn a_run_with_no_proposals_succeeds_and_stores_nothing() {
    let fixture = Fixture::new(|r| r.register_operator(op(&[], |_| Ok(())))).await;
    let version = fixture.version(b"abc").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    assert_eq!(fixture.runs().await[0].status, "succeeded");
    assert!(fixture.values().await.is_empty());
}
