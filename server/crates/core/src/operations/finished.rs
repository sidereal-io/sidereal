//! A finished run never changes: only a `running` row is ever updated.
use super::fixture::*;
use super::*;
use sidereal_plugin_abi::{FacetValue, OperatorError};
use sqlx::Executor;

async fn fixture() -> Fixture {
    Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(op(&["test.size"], |c| {
            c.propose_facet("test.size", FacetValue::Integer(3))
                .map_err(|_| OperatorError::Failed("proposal".into()))
        }));
    })
    .await
}

#[tokio::test]
async fn a_failure_recorded_after_success_changes_nothing() {
    let fixture = fixture().await;
    let version = fixture.version(b"abc").await;
    assert_eq!(fixture.run(&version).await.unwrap(), RunOutcome::Succeeded);
    let run = fixture.runs().await.remove(0);
    let outcome = record_failure(&fixture.database, run.id, VALUES_NOT_STORED)
        .await
        .unwrap();
    assert_eq!(outcome, RunOutcome::Succeeded);
    let after = fixture.runs().await;
    assert_eq!(after.len(), 1);
    assert_eq!(
        (after[0].status.as_str(), &after[0].reason),
        ("succeeded", &None)
    );
    assert_eq!(fixture.values().await.len(), 1);
}

#[tokio::test]
async fn a_second_failure_keeps_the_first_reason() {
    let fixture = Fixture::new(|r| {
        r.register_operator(op(&[], |_| Err(OperatorError::Failed("first".into()))))
    })
    .await;
    let version = fixture.version(b"abc").await;
    fixture.run(&version).await.unwrap();
    let run = fixture.runs().await.remove(0);
    assert_eq!(
        record_failure(&fixture.database, run.id, "second")
            .await
            .unwrap(),
        RunOutcome::Failed("first".into())
    );
    assert_eq!(fixture.failed_reason().await, "first");
}

#[tokio::test]
async fn values_for_a_run_that_already_ended_are_not_stored() {
    let fixture = fixture().await;
    let version = fixture.version(b"abc").await;
    fixture.run(&version).await.unwrap();
    let mut conn = fixture.conn().await;
    conn.execute("DELETE FROM public.facet_values")
        .await
        .unwrap();
    let run = fixture.runs().await.remove(0);
    let stored = store(
        &fixture.database,
        &fixture.registry,
        run.id,
        version.version.id,
        &Version {
            asset_id: version.id,
            content_hash: version.version.content_hash.clone(),
            byte_size: 3,
        },
        &FacetProposals {
            producer_id: "test.op".into(),
            producer_version: "1.2.3".into(),
            grants: vec!["test.size".into()],
            proposals: vec![sidereal_plugin_abi::FacetProposal {
                name: "test.size".into(),
                value: FacetValue::Integer(4),
            }],
        },
    )
    .await
    .unwrap();
    assert!(stored.is_err(), "{stored:?}");
    assert!(fixture.values().await.is_empty());
    assert_eq!(fixture.runs().await[0].status, "succeeded");
}

#[tokio::test]
async fn values_the_database_refuses_leave_the_run_failed() {
    let fixture = fixture().await;
    let version = fixture.version(b"abc").await;
    fixture
        .conn()
        .await
        .execute("ALTER TABLE public.facet_values ADD CONSTRAINT refuse_all CHECK (false)")
        .await
        .unwrap();
    assert_eq!(
        fixture.run(&version).await.unwrap(),
        RunOutcome::Failed(VALUES_NOT_STORED.into())
    );
    assert_eq!(fixture.failed_reason().await, VALUES_NOT_STORED);
    assert!(fixture.values().await.is_empty());
}
