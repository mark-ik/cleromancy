// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, HashMap};

use chartulary::{FacetError, FacetId};
use graphshell_protocol::{
    BoundsRelationship, CachePolicy, CardValueV1, ContentHash, PortableCardV1, PresentationBinding,
    PresentationCapability, PresentationCodec, PresentationKey, PresentationManifest,
    PresentationOffer, PresentationSemantics, ProjectionRequest, ProjectionSession,
    ProjectionSnapshot, ProtocolVersion, SemanticRole,
};
use mere::kernel::geometry::PortablePoint;
use mere::kernel::graph::apply::{GraphDelta, add_node, apply_graph_delta, assert_relation};
use mere::kernel::graph::{
    EdgeAssertion, Graph, NodeFacetStore, NodeKey, ProvenanceSubKind, RelationKind,
};
use mere::kernel::persistence::GraphSnapshot;
use muniment::{Backend, JsonSlots, StoreError};
use sceno::{
    Arrangement, Footprint, InstanceId, ProjectedItem, Rect, Representation, RoutedRelation, Scene,
    Score, Size2, SourceRef, Transform2, Vec2,
};
use scenotime::{Revision, SceneEpoch, SceneSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{ContextSnapshot, Reading};

pub const HOST_SLOT: &str = "cleromancy/mere-host/v1";
pub const LOCAL_SESSION: &str = "local:cleromancy";
pub const CONTEXT_FACET: &str = "cleromancy.context/v1";
pub const READING_FACET: &str = "cleromancy.reading/v1";

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
    pub(crate) graph: Graph,
    pub(crate) projection_epoch: u64,
    pub(crate) projection_revision: u64,
    pub(crate) resources: BTreeMap<ContentHash, Vec<u8>>,
    persisted_document: Option<PersistedHost>,
    dirty: bool,
}

impl<B: Backend> CleromancyHost<B> {
    pub fn empty(backend: B) -> Self {
        Self {
            slots: JsonSlots::new(backend),
            graph: Graph::new(),
            projection_epoch: 1,
            projection_revision: 1,
            resources: BTreeMap::new(),
            persisted_document: None,
            dirty: true,
        }
    }

    pub async fn open(backend: B) -> Result<Self, HostError> {
        let slots = JsonSlots::new(backend);
        let Some(saved): Option<PersistedHost> = slots.load(HOST_SLOT).await? else {
            return Ok(Self {
                slots,
                graph: Graph::new(),
                projection_epoch: 1,
                projection_revision: 1,
                resources: BTreeMap::new(),
                persisted_document: None,
                dirty: true,
            });
        };
        let persisted_document = saved.clone();
        let mut graph = Graph::from_snapshot(&saved.graph);
        graph.overlay_facets(saved.facets);
        Ok(Self {
            slots,
            graph,
            projection_epoch: saved.projection_epoch,
            projection_revision: saved.projection_revision,
            resources: BTreeMap::new(),
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
        ProjectionSession(LOCAL_SESSION.to_string())
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

    pub fn insert_reading(
        &mut self,
        context: &ContextSnapshot,
        reading: &Reading,
    ) -> Result<NodeKey, HostError> {
        let context_key = self.insert_context(context)?;
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
        self.changed();
        Ok(key)
    }

    pub fn facet_value(&self, key: NodeKey, facet: &str) -> Option<&Value> {
        let node = self.graph.get_node(key)?;
        self.graph.facets().get(&node.id, &FacetId::new(facet))
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
        let layout = mere::canvas::project_canvas_strategy_with_score(
            "phyllotaxis.default",
            &self.graph,
            None,
            1280,
            720,
            None,
            None,
            true,
        );
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
                        actions: Vec::new(),
                    },
                }],
            );
            resources.insert(resource, bytes);
        }

        for relation in self.graph.relations() {
            let (Some(&from), Some(&to), Some(from_position), Some(to_position)) = (
                instance_of.get(&relation.from),
                instance_of.get(&relation.to),
                positions.get(&relation.from),
                positions.get(&relation.to),
            ) else {
                continue;
            };
            scene.relations.push(RoutedRelation {
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
