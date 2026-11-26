//! A run that does not finish cleanly is recorded `failed` with a reason
//! that is never empty and holds no zero character, and stores no value.
use super::fixture::*;
use super::*;
use sidereal_plugin_abi::{AssetContext, FacetValue, Operator, OperatorError};
use std::{os::unix::fs::PermissionsExt, sync::atomic::Ordering};

/// Register `test.op`, writing `test.size`, with `script`; run it once over
/// a new version; answer the failed run's reason after checking that no
/// value was stored.
async fn failed_with(script: fn(&mut AssetContext<'_>) -> Result<(), OperatorError>) -> String {
    let fixture = Fixture::new(move |r| {
        r.declare_facet(count("test.size"));
        r.register_operator(op(&["test.size"], script));
    })
    .await;
    let version = fixture.version(b"abc").await;
    let outcome = fixture.run(&version).await.unwrap();
    let reason = fixture.failed_reason().await;
    assert_eq!(outcome, RunOutcome::Failed(reason.clone()));
    assert!(fixture.values().await.is_empty());
    reason
}

fn size(context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
    context
        .propose_facet("test.size", FacetValue::Integer(3))
        .map_err(|_| OperatorError::Failed("proposal".into()))
}

#[tokio::test]
async fn the_operators_failure_reason_is_recorded() {
    let reason = failed_with(|c| {
        size(c)?;
        Err(OperatorError::Failed("could not read the header".into()))
    })
    .await;
    assert_eq!(reason, "could not read the header");
}

#[tokio::test]
async fn an_empty_failure_reason_is_recorded_as_no_reason() {
    let reason = failed_with(|_| Err(OperatorError::Failed(String::new()))).await;
    assert_eq!(reason, "the operator gave no reason");
}

#[tokio::test]
async fn a_zero_character_is_removed_from_the_reason() {
    let reason = failed_with(|_| Err(OperatorError::Failed("bad\0header".into()))).await;
    assert_eq!(reason, "badheader");
    let reason = failed_with(|_| Err(OperatorError::Failed("\0".into()))).await;
    assert_eq!(reason, "the operator gave no reason");
}

#[tokio::test]
async fn a_proposed_name_with_a_zero_character_is_named_without_it() {
    let reason = failed_with(|c| {
        c.propose_facet("test.\0x", FacetValue::Integer(1))
            .map_err(|_| OperatorError::Failed("proposal".into()))
    })
    .await;
    assert!(reason.contains("test.x"), "{reason}");
    assert!(!reason.contains('\0'));
}

#[tokio::test]
async fn an_unsupported_operator_is_recorded() {
    let reason = failed_with(|_| Err(OperatorError::Unsupported)).await;
    assert_eq!(reason, "the operator cannot run yet");
}

#[tokio::test]
async fn a_panic_is_recorded_as_the_operator_stopped() {
    let reason = failed_with(|c| {
        size(c)?;
        panic!("the operator broke")
    })
    .await;
    assert_eq!(reason, "the operator stopped");
}

/// Proposes a valid value, then the host cancels the run.
struct CancelledAfterProposing(Arc<AtomicBool>);
impl Operator for CancelledAfterProposing {
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
        &["image/x-test"]
    }
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        size(context)?;
        self.0.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn a_run_the_host_cancels_is_recorded_as_cancelled() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let fixture = Fixture::new(move |r| {
        r.declare_facet(count("test.size"));
        r.register_operator(Box::new(CancelledAfterProposing(flag.clone())));
    })
    .await;
    let version = fixture.version(b"abc").await;
    let outcome = fixture
        .run_with(fixture.registry.clone(), "test.op", &version, cancelled)
        .await
        .unwrap();
    let reason = fixture.failed_reason().await;
    assert_eq!(outcome, RunOutcome::Failed(reason.clone()));
    assert!(reason.contains("cancelled"), "{reason}");
    assert!(fixture.values().await.is_empty());
}

static RAN: AtomicBool = AtomicBool::new(false);

#[tokio::test]
async fn a_stored_object_shorter_than_its_version_is_refused_without_running() {
    let fixture = Fixture::new(|r| {
        r.declare_facet(count("test.size"));
        r.register_operator(op(&["test.size"], |c| {
            RAN.store(true, Ordering::SeqCst);
            size(c)
        }));
    })
    .await;
    let version = fixture.version(&[7; 1000]).await;
    let path = fixture.object(&version.version.content_hash);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(10)
        .unwrap();
    let outcome = fixture.run(&version).await.unwrap();
    assert_eq!(
        outcome,
        RunOutcome::Failed("the stored object does not match its version".into())
    );
    assert_eq!(
        fixture.failed_reason().await,
        "the stored object does not match its version"
    );
    assert!(!RAN.load(Ordering::SeqCst), "the operator ran");
    assert!(fixture.values().await.is_empty());
}

#[test]
fn one_function_cleans_every_reason() {
    assert_eq!(cleaned("could not read"), "could not read");
    assert_eq!(cleaned("bad\0header"), "badheader");
    assert_eq!(cleaned(""), "the operator gave no reason");
    assert_eq!(cleaned("\0\0"), "the operator gave no reason");
    assert_eq!(cleaned("  "), "the operator gave no reason");
}
