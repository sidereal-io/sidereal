//! A database, paired storage and one Registry per test, with test plugins
//! registered through the public contract.
use super::*;
use crate::{
    assets::AssetRecord,
    database::initialize_with_storage,
    ingest::{ingest, Ingested},
    storage::{objects::ObjectStore, tests::TempRoot},
    test_support::TestDatabase,
};
use sidereal_plugin_abi::{
    AssetContext, AssetProposal, FacetSchema, FacetScope, FacetType, Operator, OperatorError, Pack,
    Source, SourceError,
};
use sqlx::{Connection, PgConnection, Row};
use std::{path::PathBuf, time::Duration};

const SECONDS: Duration = Duration::from_secs(3);

/// The Source that brings every test candidate in as an image.
struct Anything;
impl Source for Anything {
    fn id(&self) -> &str {
        "test.anything"
    }
    fn version(&self) -> &str {
        "1"
    }
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), SourceError> {
        context
            .propose_asset(AssetProposal::new("image", "image/x-test"))
            .map_err(|_| SourceError::Failed("proposal".into()))
    }
}

/// A test Operator whose run is a plain function.
pub(super) struct Scripted {
    pub id: &'static str,
    pub version: &'static str,
    pub writes: &'static [&'static str],
    pub script: fn(&mut AssetContext<'_>) -> Result<(), OperatorError>,
}
impl Operator for Scripted {
    fn id(&self) -> &str {
        self.id
    }
    fn version(&self) -> &str {
        self.version
    }
    fn writes(&self) -> &[&str] {
        self.writes
    }
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        (self.script)(context)
    }
}

/// An Operator `test.op` at version `1.2.3`.
pub(super) fn op(
    writes: &'static [&'static str],
    script: fn(&mut AssetContext<'_>) -> Result<(), OperatorError>,
) -> Box<dyn Operator> {
    Box::new(Scripted {
        id: "test.op",
        version: "1.2.3",
        writes,
        script,
    })
}

/// A count schema at version 1, in bytes, on the Asset version.
pub(super) fn count(name: &str) -> FacetSchema {
    FacetSchema {
        name: name.into(),
        label: "Size".into(),
        value_type: FacetType::Count,
        unit: "bytes".into(),
        version: 1,
        scope: FacetScope::AssetVersion,
    }
}

/// A choice schema at version 1, with no unit, on the Asset version.
pub(super) fn choice(name: &str, allowed: &[&str]) -> FacetSchema {
    FacetSchema {
        name: name.into(),
        label: "Kind".into(),
        value_type: FacetType::Choice(allowed.iter().map(|&a| a.to_owned()).collect()),
        unit: String::new(),
        version: 1,
        scope: FacetScope::AssetVersion,
    }
}

/// The pack `test`: the Source, then whatever the test registers.
pub(super) struct TestPack(pub Box<dyn Fn(&mut Registry)>);
impl Pack for TestPack {
    fn id(&self) -> &str {
        "test"
    }
    fn register(&self, registry: &mut Registry) {
        registry.register_source(Box::new(Anything));
        (self.0)(registry)
    }
}

pub(super) fn registry(register: impl Fn(&mut Registry) + 'static) -> Arc<Registry> {
    let mut registry = Registry::new();
    registry
        .register_pack(&TestPack(Box::new(register)))
        .unwrap();
    Arc::new(registry)
}

/// One stored row of `operation_runs`.
#[derive(Debug)]
pub(super) struct Run {
    pub id: Uuid,
    pub operator_id: String,
    pub operator_version: String,
    pub asset_version_id: Uuid,
    pub actor: i64,
    pub status: String,
    pub reason: Option<String>,
    pub ended_after_start: Option<bool>,
}

/// One stored row of `facet_values`, with its schema's label and unit.
#[derive(Debug)]
pub(super) struct Value {
    pub facet: String,
    pub schema_version: i32,
    pub asset_id: Option<Uuid>,
    pub asset_version_id: Option<Uuid>,
    pub value: serde_json::Value,
    pub producer_id: String,
    pub producer_version: String,
    pub run_id: Uuid,
    pub written: bool,
    pub label: String,
    pub unit: String,
}

pub(super) struct Fixture {
    pub db: TestDatabase,
    pub temp: TempRoot,
    pub database: Database,
    pub objects: ObjectStore,
    pub registry: Arc<Registry>,
    /// The `users.id` of the claimed administrator.
    pub nova: i64,
}

impl Fixture {
    pub async fn new(register: impl Fn(&mut Registry) + 'static) -> Self {
        let db = TestDatabase::new();
        let temp = TempRoot::new();
        let (database, paired, _) =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &temp.root())
                .await
                .unwrap();
        let objects = ObjectStore::open(&temp.root(), paired).unwrap();
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        let nova = sqlx::query_scalar(
            "INSERT INTO public.users (username, password_hash, role, created_at) \
             VALUES ('nova', 'hash', 'admin', now()) RETURNING id",
        )
        .fetch_one(&mut conn)
        .await
        .unwrap();
        Self {
            db,
            temp,
            database,
            objects,
            registry: registry(register),
            nova,
        }
    }

    pub async fn conn(&self) -> PgConnection {
        PgConnection::connect_with(&self.db.options).await.unwrap()
    }

    /// Store `bytes` as a new Asset with one version.
    pub async fn version(&self, bytes: &[u8]) -> AssetRecord {
        let mut staging = self.objects.begin().unwrap();
        staging.write(bytes).unwrap();
        match ingest(
            &self.database,
            &self.objects,
            self.registry.clone(),
            "test.anything",
            "nebula.png",
            staging.finish().unwrap(),
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap()
        {
            Ingested::Created(record) => record,
            Ingested::Found(_) => panic!("the bytes were already in the library"),
        }
    }

    pub fn object(&self, hash: &str) -> PathBuf {
        self.temp
            .path
            .join("store")
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(hash)
    }

    /// Run `test.op` over `version` with this fixture's Registry.
    pub async fn run(&self, version: &AssetRecord) -> Result<RunOutcome, OperationError> {
        self.run_with(self.registry.clone(), "test.op", version, Arc::default())
            .await
    }

    pub async fn run_with(
        &self,
        registry: Arc<Registry>,
        operator: &str,
        version: &AssetRecord,
        cancelled: Arc<AtomicBool>,
    ) -> Result<RunOutcome, OperationError> {
        run_operator(
            &self.database,
            &self.objects,
            registry,
            RunRequest {
                operator_id: operator.into(),
                asset_version_id: version.version.id,
                actor: self.nova,
            },
            cancelled,
        )
        .await
    }

    pub async fn runs(&self) -> Vec<Run> {
        sqlx::query(
            "SELECT id, operator_id, operator_version, asset_version_id, actor_user_id, status, \
             reason, ended_at >= started_at FROM public.operation_runs ORDER BY started_at, id",
        )
        .fetch_all(&mut self.conn().await)
        .await
        .unwrap()
        .iter()
        .map(|row| Run {
            id: row.get(0),
            operator_id: row.get(1),
            operator_version: row.get(2),
            asset_version_id: row.get(3),
            actor: row.get(4),
            status: row.get(5),
            reason: row.get(6),
            ended_after_start: row.get(7),
        })
        .collect()
    }

    /// The one run, which must have failed; its reason.
    pub async fn failed_reason(&self) -> String {
        let runs = self.runs().await;
        assert_eq!(runs.len(), 1, "{runs:?}");
        assert_eq!(runs[0].status, "failed", "{runs:?}");
        assert_eq!(runs[0].ended_after_start, Some(true));
        runs[0].reason.clone().unwrap()
    }

    pub async fn values(&self) -> Vec<Value> {
        sqlx::query(
            "SELECT v.facet, v.schema_version, v.asset_id, v.asset_version_id, v.value, \
             v.producer_id, v.producer_version, v.run_id, v.written_at <= now(), s.label, s.unit \
             FROM public.facet_values v \
             JOIN public.facet_schemas s ON s.name = v.facet AND s.version = v.schema_version \
             ORDER BY v.facet, v.written_at, v.id",
        )
        .fetch_all(&mut self.conn().await)
        .await
        .unwrap()
        .iter()
        .map(|row| Value {
            facet: row.get(0),
            schema_version: row.get(1),
            asset_id: row.get(2),
            asset_version_id: row.get(3),
            value: row.get(4),
            producer_id: row.get(5),
            producer_version: row.get(6),
            run_id: row.get(7),
            written: row.get(8),
            label: row.get(9),
            unit: row.get(10),
        })
        .collect()
    }
}
