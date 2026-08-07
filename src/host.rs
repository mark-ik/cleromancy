// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use chartulary::{FacetError, FacetId};
use graphshell_protocol::{
    ActionFormChoiceV1, BoundsRelationship, CachePolicy, CardValueV1, ContentHash, PortableCardV1,
    PresentationBinding, PresentationCapability, PresentationCodec, PresentationKey,
    PresentationManifest, PresentationOffer, PresentationSemantics, ProjectionRequest,
    ProjectionSession, ProjectionSnapshot, ProtocolVersion, SemanticRole,
};
use mere::kernel::geometry::PortablePoint;
use mere::kernel::graph::apply::{GraphDelta, add_node, apply_graph_delta, assert_relation};
use mere::kernel::graph::{
    ContainmentSubKind, EdgeAssertion, Graph, NodeFacetStore, NodeKey, ProvenanceSubKind,
    RelationKind, SemanticSubKind,
};
use mere::kernel::persistence::GraphSnapshot;
use muniment::{Backend, JsonSlots, StoreError};
use sceno::{
    Arrangement, Footprint, InstanceId, ProjectedItem, Rect, Representation, RoutedRelation, Scene,
    Score, Size2, SourceRef, Transform2, Vec2,
};
use scenotime::{Revision, SceneEpoch, SceneSnapshot};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::moirai::clotho::EntropySource;
use crate::{
    AstrologyChart, AstrologyError, AstrologyFacts, Concurrence, ConcurrenceError, ContextSnapshot,
    Field, Reading, ReadingEngine, ReadingError, ReadingSession, Reflection, SessionError,
    SpreadError, ThreeCardPosition, ThreeCardRelationKind, ThreeCardSpread,
};

pub const HOST_SLOT: &str = "cleromancy/mere-host/v1";
pub const LOCAL_SESSION: &str = "local:cleromancy";
pub const CONTEXT_FACET: &str = "cleromancy.context/v1";
pub const FIELD_FACET: &str = "cleromancy.field/v1";
pub const READING_FACET: &str = "cleromancy.reading/v1";
pub const SESSION_FACET: &str = "cleromancy.reading-session/v1";
pub const REFLECTION_FACET: &str = "cleromancy.reflection/v1";
pub const THREE_CARD_SPREAD_FACET: &str = "cleromancy.three-card-spread/v1";
pub const ASTROLOGY_CHART_FACET: &str = "cleromancy.astrology-chart/v1";
pub const ASTROLOGY_FACTS_FACET: &str = "cleromancy.astrology-facts/v1";
pub const CONCURRENCE_FACET: &str = "cleromancy.concurrence/v1";

/// State that belongs to one projection connection rather than Cleromancy's
/// saved reading graph.
///
/// A resident authority keeps one durable [`CleromancyHost`] and gives every
/// admitted endpoint one of these states. Resources and scene instances must
/// not leak from one peer into another peer's session.
#[cfg(all(feature = "graphshell-admission", not(target_arch = "wasm32")))]
pub(crate) struct CleromancyProjectionState {
    projection_session: ProjectionSession,
    resources: BTreeMap<ContentHash, Vec<u8>>,
    active_instances: HashMap<InstanceId, NodeKey>,
    last_snapshot: Option<(SceneEpoch, Revision)>,
}

#[cfg(all(feature = "graphshell-admission", not(target_arch = "wasm32")))]
impl CleromancyProjectionState {
    pub(crate) fn for_session(projection_session: ProjectionSession) -> Self {
        Self {
            projection_session,
            resources: BTreeMap::new(),
            active_instances: HashMap::new(),
            last_snapshot: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum HostError {
    #[error("Cleromancy storage: {0}")]
    Store(#[from] StoreError),
    #[error("Cleromancy facet: {0}")]
    Facet(#[from] FacetError),
    #[error("Cleromancy projection: {0}")]
    InvalidSnapshot(String),
    #[error("request names another projection session or protocol major")]
    WrongSession,
    #[error("resource was not disclosed by this session")]
    MissingResource,
    #[error("reading requires stored {kind} {digest}")]
    MissingReadingDependency { kind: &'static str, digest: String },
    #[error("stored {facet} facet does not decode: {reason}")]
    InvalidStoredFacet { facet: &'static str, reason: String },
    #[error("reading session requires stored {kind} {id}")]
    MissingSessionDependency { kind: &'static str, id: String },
    #[error("concurrence requires stored member {address}")]
    MissingConcurrenceMember { address: String },
    #[error("the system clock precedes the Unix epoch")]
    Clock,
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Spread(#[from] SpreadError),
    #[error(transparent)]
    Astrology(#[from] AstrologyError),
    #[error(transparent)]
    Concurrence(#[from] ConcurrenceError),
    #[error(transparent)]
    Reading(#[from] ReadingError),
}

#[derive(Clone, Serialize, Deserialize)]
struct PersistedHost {
    graph: GraphSnapshot,
    facets: NodeFacetStore,
    projection_epoch: u64,
    projection_revision: u64,
}

/// Cleromancy's source of truth: a Mere graph stored as one typed Muniment
/// slot and projected through Graphshell's endpoint vocabulary.
pub struct CleromancyHost<B> {
    slots: JsonSlots<B>,
    projection_session: ProjectionSession,
    pub(crate) graph: Graph,
    pub(crate) projection_epoch: u64,
    pub(crate) projection_revision: u64,
    pub(crate) resources: BTreeMap<ContentHash, Vec<u8>>,
    active_instances: HashMap<InstanceId, NodeKey>,
    last_snapshot: Option<(SceneEpoch, Revision)>,
    persisted_document: Option<PersistedHost>,
    dirty: bool,
}

impl<B: Backend> CleromancyHost<B> {
    pub fn empty(backend: B) -> Self {
        Self {
            slots: JsonSlots::new(backend),
            projection_session: ProjectionSession(LOCAL_SESSION.to_string()),
            graph: Graph::new(),
            projection_epoch: 1,
            projection_revision: 1,
            resources: BTreeMap::new(),
            active_instances: HashMap::new(),
            last_snapshot: None,
            persisted_document: None,
            dirty: true,
        }
    }

    pub async fn open(backend: B) -> Result<Self, HostError> {
        let slots = JsonSlots::new(backend);
        let Some(saved): Option<PersistedHost> = slots.load(HOST_SLOT).await? else {
            return Ok(Self {
                slots,
                projection_session: ProjectionSession(LOCAL_SESSION.to_string()),
                graph: Graph::new(),
                projection_epoch: 1,
                projection_revision: 1,
                resources: BTreeMap::new(),
                active_instances: HashMap::new(),
                last_snapshot: None,
                persisted_document: None,
                dirty: true,
            });
        };
        let persisted_document = saved.clone();
        let mut graph = Graph::from_snapshot(&saved.graph);
        graph.overlay_facets(saved.facets);
        Ok(Self {
            slots,
            projection_session: ProjectionSession(LOCAL_SESSION.to_string()),
            graph,
            projection_epoch: saved.projection_epoch,
            projection_revision: saved.projection_revision,
            resources: BTreeMap::new(),
            active_instances: HashMap::new(),
            last_snapshot: None,
            persisted_document: Some(persisted_document),
            dirty: false,
        })
    }

    pub async fn persist(&mut self, saved_at_secs: u64) -> Result<(), HostError> {
        let document = match &self.persisted_document {
            Some(document) if !self.dirty && document.graph.timestamp_secs == saved_at_secs => {
                document.clone()
            }
            _ => {
                let mut graph = self.graph.to_snapshot();
                graph.timestamp_secs = saved_at_secs;
                PersistedHost {
                    graph,
                    facets: self.graph.facets().clone(),
                    projection_epoch: self.projection_epoch,
                    projection_revision: self.projection_revision,
                }
            }
        };
        self.slots.save(HOST_SLOT, &document).await?;
        self.persisted_document = Some(document);
        self.dirty = false;
        Ok(())
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn was_reopened(&self) -> bool {
        self.persisted_document.is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.graph.nodes().next().is_none()
    }

    pub fn session(&self) -> ProjectionSession {
        self.projection_session.clone()
    }

    #[cfg(all(feature = "graphshell-admission", not(target_arch = "wasm32")))]
    pub(crate) fn swap_projection_state(&mut self, state: &mut CleromancyProjectionState) {
        std::mem::swap(&mut self.projection_session, &mut state.projection_session);
        std::mem::swap(&mut self.resources, &mut state.resources);
        std::mem::swap(&mut self.active_instances, &mut state.active_instances);
        std::mem::swap(&mut self.last_snapshot, &mut state.last_snapshot);
    }

    #[cfg(all(feature = "graphshell-admission", not(target_arch = "wasm32")))]
    pub(crate) fn current_projection_revision(&self) -> (SceneEpoch, Revision) {
        (
            SceneEpoch(self.projection_epoch),
            Revision(self.projection_revision),
        )
    }

    /// Rebind this in-memory endpoint before it serves an already-admitted
    /// Graphshell session. The durable reading graph stays local; the session
    /// name and all disclosed resources are per connection.
    #[cfg(all(feature = "graphshell-admission", not(target_arch = "wasm32")))]
    pub(crate) fn bind_projection_session(&mut self, session: ProjectionSession) {
        self.projection_session = session;
        self.resources.clear();
        self.active_instances.clear();
        self.last_snapshot = None;
    }

    pub fn local_request(&self) -> ProjectionRequest {
        ProjectionRequest {
            version: ProtocolVersion::V1,
            session: self.session(),
            score: self.score(),
        }
    }

    pub fn insert_context(&mut self, context: &ContextSnapshot) -> Result<NodeKey, HostError> {
        let address = format!("cleromancy://context/{}", context.digest());
        let key = self.upsert_node(
            &address,
            &context.label,
            context.tags.iter().map(String::as_str).chain(["context"]),
        );
        self.set_facet(key, CONTEXT_FACET, serde_json::to_value(context).unwrap())?;
        Ok(key)
    }

    /// Retain the complete candidate field as first-class graph truth. The
    /// digest-addressed node is shared by every reading made from this exact
    /// field, so replay does not depend on a catalog or caller remaining
    /// installed.
    pub fn insert_field(&mut self, field: &Field) -> Result<NodeKey, HostError> {
        let address = format!("cleromancy://field/{}", field.digest());
        let key = self.upsert_node(&address, &field.system, ["field", field.system.as_str()]);
        self.set_facet(key, FIELD_FACET, serde_json::to_value(field).unwrap())?;
        Ok(key)
    }

    /// Store an adapter-produced chart and its structured facts as separate
    /// graph truth. The facts node is explicitly generated from the chart and
    /// remains bound to its chart digest for replay.
    pub fn insert_astrology_chart(
        &mut self,
        chart: &AstrologyChart,
        orb_millidegrees: u32,
    ) -> Result<(NodeKey, NodeKey), HostError> {
        chart.validate()?;
        let facts = chart.facts(orb_millidegrees)?;
        let chart_digest = chart.digest();
        let chart_key = self.upsert_node(
            &format!("cleromancy://astrology/chart/{chart_digest}"),
            &format!("Astrology chart ({})", chart.engine),
            ["astrology", "chart", chart.engine.as_str()],
        );
        self.set_facet(
            chart_key,
            ASTROLOGY_CHART_FACET,
            serde_json::to_value(chart).unwrap(),
        )?;

        let facts_key = self.upsert_node(
            &format!("cleromancy://astrology/facts/{}", facts.digest()),
            &format!("Astrology facts ({})", chart.moment.instant_utc),
            ["astrology", "facts"],
        );
        self.set_facet(
            facts_key,
            ASTROLOGY_FACTS_FACET,
            serde_json::to_value(&facts).unwrap(),
        )?;
        assert_relation(
            &mut self.graph,
            facts_key,
            chart_key,
            EdgeAssertion::Provenance {
                sub_kind: ProvenanceSubKind::GeneratedFrom,
            },
        );
        self.changed();
        Ok((chart_key, facts_key))
    }

    /// Resolve and verify a graph-resident astrology facts node without
    /// requiring the original adapter to remain installed.
    pub fn replay_astrology_facts(
        &self,
        facts: &AstrologyFacts,
    ) -> Result<AstrologyFacts, HostError> {
        let chart = self.astrology_chart_for_digest(&facts.chart_digest)?;
        facts.verify(&chart)?;
        Ok(chart.facts(facts.orb_millidegrees)?)
    }

    pub fn astrology_chart_for_digest(&self, digest: &str) -> Result<AstrologyChart, HostError> {
        let chart: AstrologyChart = self.stored_facet(
            &format!("cleromancy://astrology/chart/{digest}"),
            ASTROLOGY_CHART_FACET,
            "astrology chart",
            digest,
        )?;
        if chart.digest() != digest {
            return Err(HostError::InvalidStoredFacet {
                facet: ASTROLOGY_CHART_FACET,
                reason: "chart digest does not match its canonical address".to_string(),
            });
        }
        Ok(chart)
    }

    pub fn astrology_facts_for_digest(&self, digest: &str) -> Result<AstrologyFacts, HostError> {
        let facts: AstrologyFacts = self.stored_facet(
            &format!("cleromancy://astrology/facts/{digest}"),
            ASTROLOGY_FACTS_FACET,
            "astrology facts",
            digest,
        )?;
        if facts.digest() != digest {
            return Err(HostError::InvalidStoredFacet {
                facet: ASTROLOGY_FACTS_FACET,
                reason: "facts digest does not match its canonical address".to_string(),
            });
        }
        Ok(facts)
    }

    /// Retain a grouping of independently produced graph values. Membership
    /// records that the values were consulted together; it does not make one
    /// member provenance for, or an explanation of, another.
    pub fn insert_concurrence(&mut self, concurrence: &Concurrence) -> Result<NodeKey, HostError> {
        concurrence.validate()?;
        let member_keys = concurrence
            .members
            .iter()
            .map(|member| {
                self.graph
                    .get_node_by_url(&member.address)
                    .map(|(key, _)| key)
                    .ok_or_else(|| HostError::MissingConcurrenceMember {
                        address: member.address.clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let key = self.upsert_node(
            &format!("cleromancy://concurrence/{}", concurrence.id),
            "Pattern occasion",
            ["concurrence", "pattern-occasion"],
        );
        self.set_facet(
            key,
            CONCURRENCE_FACET,
            serde_json::to_value(concurrence).unwrap(),
        )?;
        for member_key in member_keys {
            assert_relation(
                &mut self.graph,
                key,
                member_key,
                EdgeAssertion::Containment {
                    sub_kind: ContainmentSubKind::CollectionMember,
                },
            );
        }
        self.changed();
        Ok(key)
    }

    pub fn replay_concurrence(&self, concurrence: &Concurrence) -> Result<Concurrence, HostError> {
        concurrence.validate()?;
        let stored: Concurrence = self.stored_facet(
            &format!("cleromancy://concurrence/{}", concurrence.id),
            CONCURRENCE_FACET,
            "concurrence",
            &concurrence.id,
        )?;
        if stored != *concurrence {
            return Err(ConcurrenceError::Invalid("stored value".to_string()).into());
        }
        for member in &stored.members {
            if self.graph.get_node_by_url(&member.address).is_none() {
                return Err(HostError::MissingConcurrenceMember {
                    address: member.address.clone(),
                });
            }
        }
        Ok(stored)
    }

    /// Resolve two saved values into one new pattern occasion. Every input is
    /// replayed from graph truth before the concurrence node is created.
    pub fn create_astrology_reading_concurrence(
        &mut self,
        astrology_facts_digest: &str,
        reading_session_id: &str,
    ) -> Result<Concurrence, HostError> {
        self.validate_astrology_reading_concurrence_members(
            astrology_facts_digest,
            reading_session_id,
        )?;
        let concurrence = Concurrence::astrology_reading(
            unix_time_ms()?,
            astrology_facts_digest,
            reading_session_id,
        )?;
        self.insert_concurrence(&concurrence)?;
        Ok(concurrence)
    }

    pub(crate) fn validate_astrology_reading_concurrence_members(
        &self,
        astrology_facts_digest: &str,
        reading_session_id: &str,
    ) -> Result<(), HostError> {
        let facts = self.astrology_facts_for_digest(astrology_facts_digest)?;
        self.replay_astrology_facts(&facts)?;
        self.reading_session_for_id(reading_session_id)?;
        Ok(())
    }

    pub fn insert_reading(
        &mut self,
        context: &ContextSnapshot,
        field: &Field,
        reading: &Reading,
    ) -> Result<NodeKey, HostError> {
        let replayed = ReadingEngine::replay(context, field, &reading.receipt)?;
        if replayed != *reading {
            return Err(ReadingError::ReceiptMismatch("sealed reading".to_string()).into());
        }
        let context_key = self.insert_context(context)?;
        let field_key = self.insert_field(field)?;
        let address = format!("cleromancy://reading/{}", reading.id);
        let mode = format!("{:?}", reading.receipt.mode).to_lowercase();
        let mut tags = vec!["reading", mode.as_str(), reading.system.as_str()];
        if reading.receipt.enrichment.is_some() {
            tags.push("externally-qualified");
        }
        let key = self.upsert_node(&address, &reading.title, tags);
        self.set_facet(key, READING_FACET, serde_json::to_value(reading).unwrap())?;
        assert_relation(
            &mut self.graph,
            key,
            context_key,
            EdgeAssertion::Provenance {
                sub_kind: ProvenanceSubKind::GeneratedFrom,
            },
        );
        assert_relation(
            &mut self.graph,
            key,
            field_key,
            EdgeAssertion::Provenance {
                sub_kind: ProvenanceSubKind::GeneratedFrom,
            },
        );
        self.changed();
        Ok(key)
    }

    /// Record a new occasion for a sealed result. This leaves the result
    /// receipt untouched, so repeated calculated reads can remain separately
    /// saved even when they resolve to the same content-addressed reading.
    pub fn record_reading_session_at_with_entropy(
        &mut self,
        context: &ContextSnapshot,
        field: &Field,
        reading: &Reading,
        created_at_ms: u64,
        client_token: Option<String>,
        entropy: &mut impl EntropySource,
    ) -> Result<ReadingSession, HostError> {
        let event_nonce = event_nonce(entropy)?;
        let session = ReadingSession::single(
            created_at_ms,
            event_nonce,
            context.digest(),
            field.digest(),
            &reading.id,
            client_token,
        )?;
        self.insert_session(context, field, std::slice::from_ref(reading), &session)?;
        Ok(session)
    }

    /// Production convenience for a session timestamped at the local host.
    /// Tests and import use the explicit `*_at_with_entropy` form above.
    pub fn record_reading_session_with_entropy(
        &mut self,
        context: &ContextSnapshot,
        field: &Field,
        reading: &Reading,
        client_token: Option<String>,
        entropy: &mut impl EntropySource,
    ) -> Result<ReadingSession, HostError> {
        self.record_reading_session_at_with_entropy(
            context,
            field,
            reading,
            unix_time_ms()?,
            client_token,
            entropy,
        )
    }

    /// Cast the fixed authored three-card layout and save its ordered session
    /// plus graph-visible position and relationship metadata.
    pub fn record_three_card_spread_at_with_entropy(
        &mut self,
        context: &ContextSnapshot,
        field: &Field,
        created_at_ms: u64,
        client_token: Option<String>,
        entropy: &mut impl EntropySource,
    ) -> Result<(ReadingSession, ThreeCardSpread, Vec<Reading>), HostError> {
        let readings = (0..ThreeCardPosition::ALL.len())
            .map(|_| ReadingEngine::cast_with(context, field, entropy))
            .collect::<Result<Vec<_>, _>>()?;
        let placements = ThreeCardPosition::ALL
            .into_iter()
            .zip(&readings)
            .map(|(position, reading)| crate::ReadingPlacement {
                position: position.as_str().to_string(),
                reading_id: reading.id.clone(),
            })
            .collect();
        let session = ReadingSession::with_placements(
            created_at_ms,
            event_nonce(entropy)?,
            context.digest(),
            field.digest(),
            placements,
            client_token,
        )?;
        self.insert_session(context, field, &readings, &session)?;
        let spread = ThreeCardSpread::new(&session.id, &session.placements)?;
        self.insert_three_card_spread(&session, &readings, &spread)?;
        Ok((session, spread, readings))
    }

    pub fn record_three_card_spread_with_entropy(
        &mut self,
        context: &ContextSnapshot,
        field: &Field,
        client_token: Option<String>,
        entropy: &mut impl EntropySource,
    ) -> Result<(ReadingSession, ThreeCardSpread, Vec<Reading>), HostError> {
        self.record_three_card_spread_at_with_entropy(
            context,
            field,
            unix_time_ms()?,
            client_token,
            entropy,
        )
    }

    /// Record an immutable reflection without mutating the sealed result or
    /// the session to which it belongs.
    pub fn record_reflection_at_with_entropy(
        &mut self,
        session: &ReadingSession,
        created_at_ms: u64,
        body: impl Into<String>,
        entropy: &mut impl EntropySource,
    ) -> Result<Reflection, HostError> {
        let reflection = Reflection::new(&session.id, created_at_ms, event_nonce(entropy)?, body)?;
        self.insert_reflection(session, &reflection)?;
        Ok(reflection)
    }

    /// Production convenience for a reflection timestamped at the local host.
    pub fn record_reflection_with_entropy(
        &mut self,
        session: &ReadingSession,
        body: impl Into<String>,
        entropy: &mut impl EntropySource,
    ) -> Result<Reflection, HostError> {
        self.record_reflection_at_with_entropy(session, unix_time_ms()?, body, entropy)
    }

    /// Store a session whose references have already been validated by the
    /// caller or a sync projection. This is public so import remains a thin
    /// mapping layer instead of a second model.
    pub fn insert_session(
        &mut self,
        context: &ContextSnapshot,
        field: &Field,
        readings: &[Reading],
        session: &ReadingSession,
    ) -> Result<NodeKey, HostError> {
        self.validate_session_bindings(context, field, readings, session)?;
        let context_key = self.insert_context(context)?;
        let field_key = self.insert_field(field)?;
        let mut reading_keys = Vec::with_capacity(session.placements.len());
        for placement in &session.placements {
            let reading = readings
                .iter()
                .find(|reading| reading.id == placement.reading_id)
                .expect("session bindings were validated before insertion");
            reading_keys.push(self.insert_reading(context, field, reading)?);
        }
        let key = self.upsert_node(
            &format!("cleromancy://session/{}", session.id),
            "Reading session",
            ["reading-session"],
        );
        self.set_facet(key, SESSION_FACET, serde_json::to_value(session).unwrap())?;
        assert_relation(
            &mut self.graph,
            key,
            context_key,
            EdgeAssertion::Provenance {
                sub_kind: ProvenanceSubKind::GeneratedFrom,
            },
        );
        assert_relation(
            &mut self.graph,
            key,
            field_key,
            EdgeAssertion::Provenance {
                sub_kind: ProvenanceSubKind::GeneratedFrom,
            },
        );
        for reading_key in reading_keys {
            assert_relation(
                &mut self.graph,
                key,
                reading_key,
                EdgeAssertion::Containment {
                    sub_kind: ContainmentSubKind::CollectionMember,
                },
            );
        }
        self.changed();
        Ok(key)
    }

    /// Attach one immutable reflection to a stored session. Later edits are a
    /// new reflection node, preserving the earlier record for inspection.
    pub fn insert_reflection(
        &mut self,
        session: &ReadingSession,
        reflection: &Reflection,
    ) -> Result<NodeKey, HostError> {
        session.validate()?;
        reflection.validate()?;
        if reflection.session_id != session.id {
            return Err(SessionError::InvalidReflection("bound session id".to_string()).into());
        }
        let session_key = self.session_key(session)?;
        let key = self.upsert_node(
            &format!("cleromancy://reflection/{}", reflection.id),
            "Reading reflection",
            ["reflection"],
        );
        self.set_facet(
            key,
            REFLECTION_FACET,
            serde_json::to_value(reflection).unwrap(),
        )?;
        assert_relation(
            &mut self.graph,
            key,
            session_key,
            EdgeAssertion::Semantic {
                sub_kind: SemanticSubKind::Elaborates,
                label: Some("reflects on".to_string()),
                decay_progress: None,
            },
        );
        self.changed();
        Ok(key)
    }

    pub fn insert_three_card_spread(
        &mut self,
        session: &ReadingSession,
        readings: &[Reading],
        spread: &ThreeCardSpread,
    ) -> Result<NodeKey, HostError> {
        session.validate()?;
        spread.validate()?;
        if spread.session_id != session.id {
            return Err(SpreadError::InvalidSpread("bound session id".to_string()).into());
        }
        let session_key = self.session_key(session)?;
        let mut reading_keys = BTreeMap::new();
        for placement in &spread.placements {
            let session_placement = session
                .placements
                .iter()
                .find(|candidate| candidate.position == placement.position.as_str())
                .ok_or_else(|| SpreadError::InvalidSpread("session positions".to_string()))?;
            if session_placement.reading_id != placement.reading_id {
                return Err(SpreadError::InvalidSpread("placement reading id".to_string()).into());
            }
            let reading = readings
                .iter()
                .find(|reading| reading.id == placement.reading_id)
                .ok_or_else(|| HostError::MissingReadingDependency {
                    kind: "reading",
                    digest: placement.reading_id.clone(),
                })?;
            let key = self
                .graph
                .get_node_by_url(&format!("cleromancy://reading/{}", reading.id))
                .map(|(key, _)| key)
                .ok_or_else(|| HostError::MissingReadingDependency {
                    kind: "reading",
                    digest: reading.id.clone(),
                })?;
            reading_keys.insert(placement.position, key);
        }
        let key = self.upsert_node(
            &format!("cleromancy://spread/three-card/{}", spread.id),
            "Three-card spread",
            ["spread", "three-card-spread"],
        );
        self.set_facet(
            key,
            THREE_CARD_SPREAD_FACET,
            serde_json::to_value(spread).unwrap(),
        )?;
        assert_relation(
            &mut self.graph,
            key,
            session_key,
            EdgeAssertion::Provenance {
                sub_kind: ProvenanceSubKind::GeneratedFrom,
            },
        );
        for reading_key in reading_keys.values() {
            assert_relation(
                &mut self.graph,
                key,
                *reading_key,
                EdgeAssertion::Containment {
                    sub_kind: ContainmentSubKind::CollectionMember,
                },
            );
        }
        for relation in &spread.relations {
            assert_relation(
                &mut self.graph,
                reading_keys[&relation.from],
                reading_keys[&relation.to],
                EdgeAssertion::Semantic {
                    sub_kind: match relation.kind {
                        ThreeCardRelationKind::Questions => SemanticSubKind::Questions,
                        ThreeCardRelationKind::NextStep => SemanticSubKind::NextStep,
                    },
                    label: Some(relation.label.clone()),
                    decay_progress: None,
                },
            );
        }
        self.changed();
        Ok(key)
    }

    /// Replay a reading from graph-resident truth alone. The caller supplies
    /// neither its context nor its candidate field.
    pub fn replay_reading(&self, reading: &Reading) -> Result<Reading, HostError> {
        let context = self.stored_facet::<ContextSnapshot>(
            &format!("cleromancy://context/{}", reading.receipt.context_digest),
            CONTEXT_FACET,
            "context",
            &reading.receipt.context_digest,
        )?;
        let field = self.stored_facet::<Field>(
            &format!("cleromancy://field/{}", reading.receipt.field_digest),
            FIELD_FACET,
            "field",
            &reading.receipt.field_digest,
        )?;
        Ok(ReadingEngine::replay(&context, &field, &reading.receipt)?)
    }

    /// Resolve a stored reading occasion solely from graph-resident context,
    /// field, and sealed result nodes. The returned order is the session's
    /// declared placement order.
    pub fn replay_session(&self, session: &ReadingSession) -> Result<Vec<Reading>, HostError> {
        let stored = self.stored_facet::<ReadingSession>(
            &format!("cleromancy://session/{}", session.id),
            SESSION_FACET,
            "session",
            &session.id,
        )?;
        if stored != *session {
            return Err(SessionError::InvalidSession("stored value".to_string()).into());
        }
        let context = self.stored_facet::<ContextSnapshot>(
            &format!("cleromancy://context/{}", session.context_digest),
            CONTEXT_FACET,
            "context",
            &session.context_digest,
        )?;
        let field = self.stored_facet::<Field>(
            &format!("cleromancy://field/{}", session.field_digest),
            FIELD_FACET,
            "field",
            &session.field_digest,
        )?;
        let mut readings = Vec::with_capacity(session.placements.len());
        for placement in &session.placements {
            let reading = self.stored_facet::<Reading>(
                &format!("cleromancy://reading/{}", placement.reading_id),
                READING_FACET,
                "reading",
                &placement.reading_id,
            )?;
            readings.push(reading);
        }
        self.validate_session_bindings(&context, &field, &readings, session)?;
        Ok(readings)
    }

    /// Resolve one stored reading session and verify all of its replay
    /// dependencies before returning its exact saved identity.
    pub fn reading_session_for_id(&self, id: &str) -> Result<ReadingSession, HostError> {
        let session: ReadingSession = self.stored_facet(
            &format!("cleromancy://session/{id}"),
            SESSION_FACET,
            "session",
            id,
        )?;
        self.replay_session(&session)?;
        Ok(session)
    }

    pub fn replay_three_card_spread(
        &self,
        spread: &ThreeCardSpread,
    ) -> Result<Vec<Reading>, HostError> {
        let stored = self.stored_facet::<ThreeCardSpread>(
            &format!("cleromancy://spread/three-card/{}", spread.id),
            THREE_CARD_SPREAD_FACET,
            "spread",
            &spread.id,
        )?;
        if stored != *spread {
            return Err(SpreadError::InvalidSpread("stored value".to_string()).into());
        }
        let session = self.stored_facet::<ReadingSession>(
            &format!("cleromancy://session/{}", spread.session_id),
            SESSION_FACET,
            "session",
            &spread.session_id,
        )?;
        let readings = self.replay_session(&session)?;
        let mut ordered = Vec::with_capacity(spread.placements.len());
        for placement in &spread.placements {
            ordered.push(
                readings
                    .iter()
                    .find(|reading| reading.id == placement.reading_id)
                    .ok_or_else(|| HostError::MissingReadingDependency {
                        kind: "reading",
                        digest: placement.reading_id.clone(),
                    })?
                    .clone(),
            );
        }
        Ok(ordered)
    }

    pub fn facet_value(&self, key: NodeKey, facet: &str) -> Option<&Value> {
        let node = self.graph.get_node(key)?;
        self.graph.facets().get(&node.id, &FacetId::new(facet))
    }

    fn stored_facet<T: DeserializeOwned>(
        &self,
        address: &str,
        facet: &'static str,
        kind: &'static str,
        digest: &str,
    ) -> Result<T, HostError> {
        let (key, _) = self.graph.get_node_by_url(address).ok_or_else(|| {
            HostError::MissingReadingDependency {
                kind,
                digest: digest.to_string(),
            }
        })?;
        let value =
            self.facet_value(key, facet)
                .ok_or_else(|| HostError::MissingReadingDependency {
                    kind,
                    digest: digest.to_string(),
                })?;
        serde_json::from_value(value.clone()).map_err(|error| HostError::InvalidStoredFacet {
            facet,
            reason: error.to_string(),
        })
    }

    fn session_key(&self, session: &ReadingSession) -> Result<NodeKey, HostError> {
        let (key, _) = self
            .graph
            .get_node_by_url(&format!("cleromancy://session/{}", session.id))
            .ok_or_else(|| HostError::MissingSessionDependency {
                kind: "session",
                id: session.id.clone(),
            })?;
        let stored = self.facet_value(key, SESSION_FACET).ok_or_else(|| {
            HostError::MissingSessionDependency {
                kind: "session",
                id: session.id.clone(),
            }
        })?;
        let stored: ReadingSession = serde_json::from_value(stored.clone()).map_err(|error| {
            HostError::InvalidStoredFacet {
                facet: SESSION_FACET,
                reason: error.to_string(),
            }
        })?;
        if stored != *session {
            return Err(SessionError::InvalidSession("stored value".to_string()).into());
        }
        Ok(key)
    }

    fn validate_session_bindings(
        &self,
        context: &ContextSnapshot,
        field: &Field,
        readings: &[Reading],
        session: &ReadingSession,
    ) -> Result<(), HostError> {
        session.validate()?;
        if session.context_digest != context.digest() {
            return Err(SessionError::InvalidSession("bound context digest".to_string()).into());
        }
        if session.field_digest != field.digest() {
            return Err(SessionError::InvalidSession("bound field digest".to_string()).into());
        }
        for placement in &session.placements {
            let reading = readings
                .iter()
                .find(|reading| reading.id == placement.reading_id)
                .ok_or_else(|| HostError::MissingSessionDependency {
                    kind: "reading",
                    id: placement.reading_id.clone(),
                })?;
            if reading.receipt.context_digest != session.context_digest {
                return Err(
                    SessionError::InvalidSession("reading context digest".to_string()).into(),
                );
            }
            if reading.receipt.field_digest != session.field_digest {
                return Err(
                    SessionError::InvalidSession("reading field digest".to_string()).into(),
                );
            }
            let replayed = ReadingEngine::replay(context, field, &reading.receipt)?;
            if replayed != *reading {
                return Err(ReadingError::ReceiptMismatch("sealed reading".to_string()).into());
            }
        }
        Ok(())
    }

    pub(crate) fn active_revision(&self) -> Option<(SceneEpoch, Revision)> {
        self.last_snapshot.map(|_| {
            (
                SceneEpoch(self.projection_epoch),
                Revision(self.projection_revision),
            )
        })
    }

    /// The concrete revision this projection session last rendered.
    ///
    /// Unlike [`Self::active_revision`], this does not follow later durable
    /// graph changes. Resident endpoints use it to decide whether another
    /// admitted session advanced their view.
    #[cfg(all(feature = "graphshell-admission", not(target_arch = "wasm32")))]
    pub(crate) fn last_snapshot_revision(&self) -> Option<(SceneEpoch, Revision)> {
        self.last_snapshot
    }

    pub(crate) fn context_for_instance(&self, instance: InstanceId) -> Option<ContextSnapshot> {
        let key = *self.active_instances.get(&instance)?;
        serde_json::from_value(self.facet_value(key, CONTEXT_FACET)?.clone()).ok()
    }

    pub(crate) fn field_for_digest(&self, digest: &str) -> Result<Field, HostError> {
        let address = format!("cleromancy://field/{digest}");
        let key = self
            .graph
            .get_node_by_url(&address)
            .map(|(key, _)| key)
            .ok_or_else(|| HostError::MissingReadingDependency {
                kind: "field",
                digest: digest.to_string(),
            })?;
        let value = self.facet_value(key, FIELD_FACET).ok_or_else(|| {
            HostError::MissingReadingDependency {
                kind: "field",
                digest: digest.to_string(),
            }
        })?;
        let field = serde_json::from_value::<Field>(value.clone()).map_err(|error| {
            HostError::InvalidStoredFacet {
                facet: FIELD_FACET,
                reason: error.to_string(),
            }
        })?;
        if field.digest() != digest {
            return Err(HostError::InvalidStoredFacet {
                facet: FIELD_FACET,
                reason: "field digest does not match its canonical address".to_string(),
            });
        }
        Ok(field)
    }

    pub(crate) fn intent_was_advertised(&self, instance: InstanceId, intent: &str) -> bool {
        if crate::intents::scope_for(intent).is_none() {
            return false;
        }
        let Some(key) = self.active_instances.get(&instance).copied() else {
            return false;
        };
        if intent == crate::intents::CREATE_CONCURRENCE_INTENT {
            let forms_are_available = self
                .concurrence_form_choices()
                .map(|(facts, sessions)| !facts.is_empty() && !sessions.is_empty())
                .unwrap_or(false);
            return forms_are_available
                && (self.facet_value(key, ASTROLOGY_FACTS_FACET).is_some()
                    || self.facet_value(key, SESSION_FACET).is_some());
        }
        self.facet_value(key, CONTEXT_FACET).is_some()
    }

    /// Offer only exact saved values that replay against graph truth. The
    /// action form does not encode a correspondence or choose a counterpart:
    /// it makes both member identities an explicit, bounded host selection.
    fn concurrence_form_choices(
        &self,
    ) -> Result<(Vec<ActionFormChoiceV1>, Vec<ActionFormChoiceV1>), HostError> {
        let mut facts_choices = Vec::new();
        let mut session_choices = Vec::new();
        for (key, _) in self.graph.nodes() {
            if let Some(value) = self.facet_value(key, ASTROLOGY_FACTS_FACET) {
                let facts =
                    serde_json::from_value::<AstrologyFacts>(value.clone()).map_err(|error| {
                        HostError::InvalidStoredFacet {
                            facet: ASTROLOGY_FACTS_FACET,
                            reason: error.to_string(),
                        }
                    })?;
                let facts = self.replay_astrology_facts(&facts)?;
                let digest = facts.digest();
                facts_choices.push(
                    ActionFormChoiceV1::new(
                        digest.clone(),
                        format!("Astrology facts {}", short_identity(&digest)),
                    )
                    .with_description(format!(
                        "{} · {} placements · {} aspects",
                        facts.algorithm,
                        facts.placements.len(),
                        facts.aspects.len()
                    )),
                );
            }
            if let Some(value) = self.facet_value(key, SESSION_FACET) {
                let session =
                    serde_json::from_value::<ReadingSession>(value.clone()).map_err(|error| {
                        HostError::InvalidStoredFacet {
                            facet: SESSION_FACET,
                            reason: error.to_string(),
                        }
                    })?;
                self.replay_session(&session)?;
                session_choices.push(
                    ActionFormChoiceV1::new(
                        session.id.clone(),
                        format!("Reading session {}", short_identity(&session.id)),
                    )
                    .with_description(format!(
                        "{} placement(s) · recorded {} ms since Unix epoch",
                        session.placements.len(),
                        session.created_at_ms
                    )),
                );
            }
        }
        facts_choices.sort_by(|left, right| left.value.cmp(&right.value));
        session_choices.sort_by(|left, right| left.value.cmp(&right.value));
        Ok((facts_choices, session_choices))
    }

    pub(crate) fn concurrence_target_matches(
        &self,
        instance: InstanceId,
        astrology_facts_digest: &str,
        reading_session_id: &str,
    ) -> bool {
        let Some(key) = self.active_instances.get(&instance).copied() else {
            return false;
        };
        let Some(node) = self.graph.get_node(key) else {
            return false;
        };
        node.url() == format!("cleromancy://astrology/facts/{astrology_facts_digest}")
            || node.url() == format!("cleromancy://session/{reading_session_id}")
    }

    fn upsert_node<'a>(
        &mut self,
        address: &str,
        title: &str,
        tags: impl IntoIterator<Item = &'a str>,
    ) -> NodeKey {
        let key = self
            .graph
            .get_node_by_url(address)
            .map(|(key, _)| key)
            .unwrap_or_else(|| {
                add_node(
                    &mut self.graph,
                    Some(Graph::node_namespace_id(address)),
                    address.to_string(),
                    PortablePoint::new(0.0, 0.0),
                )
            });
        apply_graph_delta(
            &mut self.graph,
            GraphDelta::SetNodeTitle {
                key,
                title: title.to_string(),
            },
        );
        for tag in tags {
            apply_graph_delta(
                &mut self.graph,
                GraphDelta::InsertNodeTag {
                    key,
                    tag: tag.to_string(),
                },
            );
        }
        self.changed();
        key
    }

    fn set_facet(&mut self, key: NodeKey, facet: &str, value: Value) -> Result<(), HostError> {
        apply_graph_delta(
            &mut self.graph,
            GraphDelta::SetNodeFacet {
                key,
                facet: facet.to_string(),
                value,
            },
        );
        self.changed();
        Ok(())
    }

    fn changed(&mut self) {
        self.projection_revision = self.projection_revision.wrapping_add(1);
        self.resources.clear();
        self.dirty = true;
    }

    fn score(&self) -> Score {
        mere::canvas::project_canvas_strategy_with_score(
            "phyllotaxis.default",
            &self.graph,
            None,
            1280,
            720,
            None,
            None,
            true,
        )
        .score
        .unwrap_or_else(|| Score::new(Arrangement::Spiral(Default::default())))
    }

    pub(crate) fn build_snapshot(&mut self) -> Result<ProjectionSnapshot, HostError> {
        self.build_snapshot_with_actions(false)
    }

    pub(crate) fn build_snapshot_with_actions(
        &mut self,
        advertise_intents: bool,
    ) -> Result<ProjectionSnapshot, HostError> {
        let (concurrence_facts, concurrence_sessions) = if advertise_intents {
            self.concurrence_form_choices()?
        } else {
            (Vec::new(), Vec::new())
        };
        let mut layout = mere::canvas::project_canvas_strategy_with_score(
            "phyllotaxis.default",
            &self.graph,
            None,
            1280,
            720,
            None,
            None,
            true,
        );
        let mut projection_keys = layout
            .positions
            .iter()
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        // Phyllotaxis supplies a stable point set but currently emits it
        // through a hash map. Pair canonical graph identities with a
        // canonical point order before assigning Graphshell instances.
        projection_keys.sort_by_key(|key| {
            self.graph
                .get_node(*key)
                .expect("layout key remains in graph")
                .id
        });
        let mut projection_points = layout
            .positions
            .iter()
            .map(|(_, point)| *point)
            .collect::<Vec<_>>();
        projection_points.sort_by(|left, right| {
            left.x
                .total_cmp(&right.x)
                .then_with(|| left.y.total_cmp(&right.y))
        });
        layout.positions = projection_keys.into_iter().zip(projection_points).collect();
        let mut scene = Scene::new();
        let mut presentation = PresentationManifest::default();
        let mut resources = BTreeMap::new();
        let mut instance_of = HashMap::with_capacity(layout.positions.len());
        let positions = layout.positions.iter().copied().collect::<HashMap<_, _>>();
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for (index, (key, position)) in layout.positions.iter().copied().enumerate() {
            let node = self
                .graph
                .get_node(key)
                .expect("layout key remains in graph");
            let instance = InstanceId(index as u32);
            instance_of.insert(key, instance);
            min_x = min_x.min(position.x);
            min_y = min_y.min(position.y);
            max_x = max_x.max(position.x);
            max_y = max_y.max(position.y);
            let source =
                scene.intern_source(SourceRef::new("cleromancy.graph", node.id.to_string()));
            scene.items.push(ProjectedItem {
                source,
                space: Scene::WORLD,
                transform: Transform2::translation(position.x, position.y),
                footprint: Footprint::Rect {
                    size: Size2::new(260.0, 150.0),
                },
                representation: Representation::Card,
                layer: 0,
                visible: true,
                hit: None,
                channels: Vec::new(),
            });

            let card = self.card_for(key);
            let bytes = serde_json::to_vec(&card).expect("portable card serializes");
            let resource = ContentHash::of(&bytes);
            let key_ref = PresentationKey(format!("cleromancy:{}", node.id));
            presentation.bindings.push(PresentationBinding {
                instance,
                key: key_ref.clone(),
            });
            presentation.offers.insert(
                key_ref,
                vec![PresentationOffer {
                    codec: PresentationCodec::PortableCardV1,
                    resource,
                    byte_size: bytes.len() as u64,
                    requires: PresentationCapability::PortableCard,
                    semantics: PresentationSemantics {
                        label: node.title.clone(),
                        role: SemanticRole::Article,
                        bounds: BoundsRelationship::FillFootprint,
                        actions: if !advertise_intents {
                            Vec::new()
                        } else if self.facet_value(key, CONTEXT_FACET).is_some() {
                            crate::intents::advertised_actions()
                        } else if self.facet_value(key, ASTROLOGY_FACTS_FACET).is_some()
                            || self.facet_value(key, SESSION_FACET).is_some()
                        {
                            crate::intents::concurrence_actions(
                                &concurrence_facts,
                                &concurrence_sessions,
                            )
                        } else {
                            Vec::new()
                        },
                    },
                }],
            );
            resources.insert(resource, bytes);
        }

        let mut routed_relations = Vec::new();
        for relation in self.graph.relations() {
            let (Some(&from), Some(&to), Some(from_position), Some(to_position)) = (
                instance_of.get(&relation.from),
                instance_of.get(&relation.to),
                positions.get(&relation.from),
                positions.get(&relation.to),
            ) else {
                continue;
            };
            routed_relations.push(RoutedRelation {
                from,
                to,
                space: Scene::WORLD,
                points: vec![
                    Vec2::new(from_position.x, from_position.y),
                    Vec2::new(to_position.x, to_position.y),
                ],
                kind: Some(relation_kind_label(relation.kind).to_string()),
                weight: Some(1.0),
            });
        }
        routed_relations.sort_by(|left, right| {
            left.from
                .0
                .cmp(&right.from.0)
                .then_with(|| left.to.0.cmp(&right.to.0))
                .then_with(|| left.kind.cmp(&right.kind))
        });
        scene.relations = routed_relations;

        scene.bounds = if layout.positions.is_empty() {
            Rect::new(Vec2::new(0.0, 0.0), Size2::new(0.0, 0.0))
        } else {
            Rect::new(
                Vec2::new(min_x - 130.0, min_y - 75.0),
                Size2::new(max_x - min_x + 260.0, max_y - min_y + 150.0),
            )
        };
        scene.generation = self.projection_revision;
        let scene = SceneSnapshot::from_dense(
            SceneEpoch(self.projection_epoch),
            Revision(self.projection_revision),
            scene,
        )
        .map_err(|error| HostError::InvalidSnapshot(format!("{error:?}")))?;
        self.active_instances = instance_of
            .into_iter()
            .map(|(key, instance)| (instance, key))
            .collect();
        self.last_snapshot = Some((scene.epoch, scene.revision));
        self.resources = resources;
        Ok(ProjectionSnapshot {
            version: ProtocolVersion::V1,
            session: self.session(),
            scene,
            presentation,
            cache_policy: CachePolicy::default(),
        })
    }

    fn card_for(&self, key: NodeKey) -> PortableCardV1 {
        let node = self.graph.get_node(key).expect("card key remains in graph");
        let mut tags = node.tags.iter().cloned().collect::<Vec<_>>();
        tags.sort();
        let mut values = vec![CardValueV1 {
            label: "Address".to_string(),
            value: node.url().to_string(),
        }];
        if let Some(value) = self.facet_value(key, READING_FACET) {
            if let Ok(reading) = serde_json::from_value::<Reading>(value.clone()) {
                let enrichment_values = reading.receipt.enrichment.as_ref().map(|qualification| {
                    let evidence = &qualification.evidence;
                    vec![
                        CardValueV1 {
                            label: "External source".to_string(),
                            value: format!(
                                "{} / {}",
                                evidence.endpoint_label, evidence.projection_label
                            ),
                        },
                        CardValueV1 {
                            label: "Evidence digest".to_string(),
                            value: evidence.evidence_digest.clone(),
                        },
                        CardValueV1 {
                            label: "Evidence cards".to_string(),
                            value: evidence
                                .sources
                                .iter()
                                .map(|source| source.presentation.as_str())
                                .collect::<Vec<_>>()
                                .join("; "),
                        },
                        CardValueV1 {
                            label: "External matches".to_string(),
                            value: format!("{:?}", qualification.candidate_terms),
                        },
                        CardValueV1 {
                            label: "External additions".to_string(),
                            value: format!("{:?}", qualification.weight_additions),
                        },
                    ]
                });
                values.extend([
                    CardValueV1 {
                        label: "Mode".to_string(),
                        value: format!("{:?}", reading.receipt.mode).to_lowercase(),
                    },
                    CardValueV1 {
                        label: "System".to_string(),
                        value: reading.system,
                    },
                    CardValueV1 {
                        label: "Interpretation".to_string(),
                        value: reading.interpretation,
                    },
                    CardValueV1 {
                        label: "Weights".to_string(),
                        value: format!("{:?}", reading.receipt.qualified_weights),
                    },
                    CardValueV1 {
                        label: "Sample".to_string(),
                        value: reading
                            .receipt
                            .sample
                            .map_or_else(|| "not used".to_string(), |sample| sample.to_string()),
                    },
                ]);
                if let Some(enrichment_values) = enrichment_values {
                    values.extend(enrichment_values);
                }
            }
        } else if let Some(value) = self.facet_value(key, SESSION_FACET)
            && let Ok(session) = serde_json::from_value::<ReadingSession>(value.clone())
        {
            values.extend([
                CardValueV1 {
                    label: "Recorded at".to_string(),
                    value: format!("{} ms since Unix epoch", session.created_at_ms),
                },
                CardValueV1 {
                    label: "Placements".to_string(),
                    value: session
                        .placements
                        .iter()
                        .map(|placement| {
                            format!("{}: {}", placement.position, placement.reading_id)
                        })
                        .collect::<Vec<_>>()
                        .join("; "),
                },
                CardValueV1 {
                    label: "Client token".to_string(),
                    value: session
                        .client_token
                        .unwrap_or_else(|| "not supplied".to_string()),
                },
            ]);
        } else if let Some(value) = self.facet_value(key, THREE_CARD_SPREAD_FACET)
            && let Ok(spread) = serde_json::from_value::<ThreeCardSpread>(value.clone())
        {
            values.extend([
                CardValueV1 {
                    label: "Session".to_string(),
                    value: spread.session_id,
                },
                CardValueV1 {
                    label: "Placements".to_string(),
                    value: spread
                        .placements
                        .iter()
                        .map(|placement| {
                            format!("{}: {}", placement.position.as_str(), placement.reading_id)
                        })
                        .collect::<Vec<_>>()
                        .join("; "),
                },
                CardValueV1 {
                    label: "Relationships".to_string(),
                    value: spread
                        .relations
                        .iter()
                        .map(|relation| {
                            format!(
                                "{} -> {} ({})",
                                relation.from.as_str(),
                                relation.to.as_str(),
                                relation.label
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; "),
                },
            ]);
        } else if let Some(value) = self.facet_value(key, REFLECTION_FACET)
            && let Ok(reflection) = serde_json::from_value::<Reflection>(value.clone())
        {
            values.extend([
                CardValueV1 {
                    label: "Recorded at".to_string(),
                    value: format!("{} ms since Unix epoch", reflection.created_at_ms),
                },
                CardValueV1 {
                    label: "Session".to_string(),
                    value: reflection.session_id,
                },
                CardValueV1 {
                    label: "Reflection".to_string(),
                    value: reflection.body,
                },
            ]);
        } else if let Some(value) = self.facet_value(key, CONCURRENCE_FACET)
            && let Ok(concurrence) = serde_json::from_value::<Concurrence>(value.clone())
        {
            values.extend([
                CardValueV1 {
                    label: "Label".to_string(),
                    value: concurrence.label,
                },
                CardValueV1 {
                    label: "Recorded at".to_string(),
                    value: format!("{} ms since Unix epoch", concurrence.created_at_ms),
                },
                CardValueV1 {
                    label: "Members".to_string(),
                    value: concurrence
                        .members
                        .iter()
                        .map(|member| format!("{}: {}", member.role, member.address))
                        .collect::<Vec<_>>()
                        .join("; "),
                },
                CardValueV1 {
                    label: "Claim".to_string(),
                    value: "Consulted together; no causal or interpretive relation asserted"
                        .to_string(),
                },
            ]);
        } else if let Some(value) = self.facet_value(key, ASTROLOGY_CHART_FACET)
            && let Ok(chart) = serde_json::from_value::<AstrologyChart>(value.clone())
        {
            values.extend([
                CardValueV1 {
                    label: "Digest".to_string(),
                    value: chart.digest(),
                },
                CardValueV1 {
                    label: "Algorithm".to_string(),
                    value: chart.algorithm,
                },
                CardValueV1 {
                    label: "Engine".to_string(),
                    value: chart.engine,
                },
                CardValueV1 {
                    label: "Ephemeris".to_string(),
                    value: chart.ephemeris,
                },
                CardValueV1 {
                    label: "Moment".to_string(),
                    value: chart.moment.instant_utc,
                },
                CardValueV1 {
                    label: "Positions".to_string(),
                    value: chart
                        .positions
                        .iter()
                        .map(|position| {
                            format!(
                                "{}: {} mdeg longitude, {} mdeg latitude{}",
                                position.body,
                                position.longitude_millidegrees,
                                position.latitude_millidegrees,
                                position
                                    .retrograde
                                    .map_or_else(String::new, |value| format!(
                                        ", retrograde={value}"
                                    )),
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; "),
                },
            ]);
        } else if let Some(value) = self.facet_value(key, ASTROLOGY_FACTS_FACET)
            && let Ok(facts) = serde_json::from_value::<AstrologyFacts>(value.clone())
        {
            values.extend([
                CardValueV1 {
                    label: "Digest".to_string(),
                    value: facts.digest(),
                },
                CardValueV1 {
                    label: "Chart".to_string(),
                    value: facts.chart_digest,
                },
                CardValueV1 {
                    label: "Algorithm".to_string(),
                    value: facts.algorithm,
                },
                CardValueV1 {
                    label: "Orb".to_string(),
                    value: format!("{} millidegrees", facts.orb_millidegrees),
                },
                CardValueV1 {
                    label: "Placements".to_string(),
                    value: facts
                        .placements
                        .iter()
                        .map(|placement| {
                            format!(
                                "{}: {:?} +{} mdeg",
                                placement.body, placement.sign, placement.degree_millidegrees
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; "),
                },
                CardValueV1 {
                    label: "Aspects".to_string(),
                    value: facts
                        .aspects
                        .iter()
                        .map(|aspect| {
                            format!(
                                "{} / {}: {:?} ({} mdeg, orb {})",
                                aspect.first,
                                aspect.second,
                                aspect.kind,
                                aspect.separation_millidegrees,
                                aspect.orb_millidegrees
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; "),
                },
            ]);
        } else if let Some(value) = self.facet_value(key, FIELD_FACET)
            && let Ok(field) = serde_json::from_value::<Field>(value.clone())
        {
            values.extend([
                CardValueV1 {
                    label: "Digest".to_string(),
                    value: field.digest(),
                },
                CardValueV1 {
                    label: "System".to_string(),
                    value: field.system,
                },
                CardValueV1 {
                    label: "Rules".to_string(),
                    value: field.rules,
                },
                CardValueV1 {
                    label: "Candidates".to_string(),
                    value: field
                        .candidates
                        .iter()
                        .map(|candidate| format!("{} ({})", candidate.title, candidate.base_weight))
                        .collect::<Vec<_>>()
                        .join("; "),
                },
            ]);
        } else if let Some(value) = self.facet_value(key, CONTEXT_FACET)
            && let Ok(context) = serde_json::from_value::<ContextSnapshot>(value.clone())
        {
            values.extend([
                CardValueV1 {
                    label: "Schema".to_string(),
                    value: context.schema,
                },
                CardValueV1 {
                    label: "Facts".to_string(),
                    value: context
                        .facts
                        .iter()
                        .map(|(name, value)| format!("{name}: {value}"))
                        .collect::<Vec<_>>()
                        .join("; "),
                },
            ]);
        }
        PortableCardV1 {
            title: node.title.clone(),
            values,
            badges: tags,
            media: Vec::new(),
        }
    }
}

fn event_nonce(entropy: &mut impl EntropySource) -> Result<String, HostError> {
    let high = entropy.next_u64()?;
    let low = entropy.next_u64()?;
    Ok(format!("{high:016x}{low:016x}"))
}

fn short_identity(value: &str) -> String {
    value.chars().take(12).collect()
}

fn unix_time_ms() -> Result<u64, HostError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| HostError::Clock)?;
    u64::try_from(duration.as_millis()).map_err(|_| HostError::Clock)
}

fn relation_kind_label(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::Semantic(_) => "semantic",
        RelationKind::Traversal => "traversal",
        RelationKind::Containment(_) => "containment",
        RelationKind::Arrangement(_) => "arrangement",
        RelationKind::Imported(_) => "imported",
        RelationKind::Provenance(_) => "provenance",
    }
}
