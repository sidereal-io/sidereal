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

/// A run keeps its schema versions in name order, whatever order it proposed
/// them in, so two runs that first write the same schemas cannot deadlock.
#[tokio::test]
async fn a_run_keeps_its_schema_versions_in_name_order() {
    use sqlx::Connection;
    use std::time::Duration;
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.a"));
        r.declare_facet(count("test.b"));
        r.register_operator(op(&["test.a", "test.b"], |context| {
            for name in ["test.b", "test.a"] {
                context
                    .propose_facet(name, FacetValue::Integer(1))
                    .map_err(|_| OperatorError::Failed("p".into()))?;
            }
            Ok(())
        }));
    })
    .await;
    let version = fixture.version(b"abc").await;
    // Another run's transaction holds an uncommitted `test.b` row.
    let mut other = fixture.conn().await;
    let mut held = other.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO public.facet_schemas VALUES \
         ('test.b', 1, 'test', 'Size', 'count', 'bytes', 'asset_version')",
    )
    .execute(&mut *held)
    .await
    .unwrap();
    let (outcome, took_a_first) = tokio::join!(fixture.run(&version), async {
        // A run that ends before it waits on a lock fails the test here.
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut watcher = fixture.conn().await;
            loop {
                let waiting: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM pg_stat_activity \
                     WHERE wait_event_type = 'Lock' AND datname = current_database()",
                )
                .fetch_one(&mut watcher)
                .await
                .unwrap();
                if waiting > 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the run never waited on test.b");
        // The run waits on `test.b`. It already holds `test.a` only if it
        // keeps schemas in name order.
        sqlx::query("SET LOCAL lock_timeout = '500ms'")
            .execute(&mut *held)
            .await
            .unwrap();
        let took_a_first = sqlx::query(
            "INSERT INTO public.facet_schemas VALUES \
             ('test.a', 1, 'test', 'Size', 'count', 'bytes', 'asset_version')",
        )
        .execute(&mut *held)
        .await
        .is_err();
        held.rollback().await.unwrap();
        took_a_first
    });
    assert!(
        took_a_first,
        "the run waited on test.b before it held test.a"
    );
    assert_eq!(outcome.unwrap(), RunOutcome::Succeeded);
    assert_eq!(fixture.values().await.len(), 2);
}

#[tokio::test]
async fn an_allowed_choice_is_stored_as_a_json_string() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(choice("test.kind", &["PNG", "JPEG"]));
        r.register_operator(op(&["test.kind"], |context| {
            context
                .propose_facet("test.kind", FacetValue::Text("PNG".into()))
                .map_err(|_| OperatorError::Failed("p".into()))
        }));
    })
    .await;
    let version = fixture.version(b"abc").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    let values = fixture.values().await;
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].value, serde_json::json!("PNG"));
}

#[tokio::test]
async fn a_new_version_that_adds_an_allowed_value_stores_it() {
    let fixture = Fixture::new(|_| {}).await;
    let version = fixture.version(b"abc").await;
    let first = registry(|r| {
        r.declare_facet(choice("test.kind", &["a", "b"]));
        r.register_operator(op(&["test.kind"], |context| {
            context
                .propose_facet("test.kind", FacetValue::Text("a".into()))
                .map_err(|_| OperatorError::Failed("p".into()))
        }));
    });
    let second = registry(|r| {
        r.declare_facet(FacetSchema {
            version: 2,
            ..choice("test.kind", &["a", "b", "c"])
        });
        r.register_operator(op(&["test.kind"], |context| {
            context
                .propose_facet("test.kind", FacetValue::Text("c".into()))
                .map_err(|_| OperatorError::Failed("p".into()))
        }));
    });
    for registry in [first, second] {
        let outcome = fixture
            .run_with(registry, "test.op", &version, Arc::default())
            .await
            .unwrap();
        assert_eq!(outcome, RunOutcome::Succeeded);
    }
    let stored: Vec<_> = fixture
        .values()
        .await
        .into_iter()
        .map(|v| (v.schema_version, v.value))
        .collect();
    assert_eq!(
        stored,
        [(1, serde_json::json!("a")), (2, serde_json::json!("c"))]
    );
    let kept: Vec<(i32, Vec<String>)> =
        sqlx::query_as("SELECT version, allowed_values FROM public.facet_schemas ORDER BY version")
            .fetch_all(&mut fixture.conn().await)
            .await
            .unwrap();
    assert_eq!(
        kept,
        [
            (1, vec!["a".to_owned(), "b".to_owned()]),
            (2, vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
        ]
    );
}
