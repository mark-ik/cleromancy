// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Context-qualified deterministic and cleromantic readings over a Mere graph.

#[cfg(all(feature = "graphshell-admission", not(target_arch = "wasm32")))]
mod admitted;
pub mod app;
pub mod astrology;
pub mod composer;
pub mod concurrence;
pub mod context;
pub mod enrichment;
pub mod field;
pub mod host;
pub mod intents;
pub mod moirai;
mod projection;
pub mod reading;
pub mod servitors;
pub mod session;
pub mod spread;
#[cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]
pub mod sync;
pub mod tarot;

pub use app::{AppError, CleromancyApp};
#[cfg(all(feature = "graphshell-admission", not(target_arch = "wasm32")))]
pub use admitted::{CleromancySessionAuthority, CleromancySessionEndpoint};
pub use astrology::{
    ASTROLOGY_CHART_SCHEMA, ASTROLOGY_FACTS_ALGORITHM, ASTROLOGY_FACTS_SCHEMA, AspectKind,
    AstrologyAdapter, AstrologyAspect, AstrologyChart, AstrologyError, AstrologyFacts,
    AstrologyMoment, AstrologyPlacement, AstrologyPosition, ZodiacSign, calculate_with_adapter,
};
pub use composer::{ComposerError, FIELD_COMPOSER_SCHEMA, FieldComposer};
pub use concurrence::{
    ASTROLOGY_FACTS_ROLE, CONCURRENCE_SCHEMA, Concurrence, ConcurrenceError, ConcurrenceMember,
    READING_SESSION_ROLE,
};
pub use context::ContextSnapshot;
pub use enrichment::{
    EnrichmentMatch, EnrichmentReport, EnrichmentSource, EnrichmentValue, ExternalProjection,
    SealedEnrichment,
};
pub use field::{
    CONTEXTUAL_WEIGHT_RULE, Candidate, EXTERNAL_TERM_WEIGHT_RULE, Field, UNIFORM_DIE_RULE,
    UNIFORM_RULE,
};
pub use host::{
    ASTROLOGY_CHART_FACET, ASTROLOGY_FACTS_FACET, CONCURRENCE_FACET, CleromancyHost, HostError,
};
pub use intents::{
    AstrologyReadingConcurrenceIntentPayload, COMPOSE_READING_INTENT, COMPOSE_READING_SCHEMA,
    COMPOSE_READING_SCOPE, CREATE_CONCURRENCE_INTENT, CREATE_CONCURRENCE_SCHEMA,
    CREATE_CONCURRENCE_SCOPE, CompositionLayout, FieldSelection, IntentLimits, READ_INTENT,
    READ_SCHEMA, READ_SCOPE, ROLL_INTENT, ROLL_SCHEMA, ROLL_SCOPE, ReadingCompositionIntentPayload,
    ReadingIntentPayload, RollIntentPayload, SELECT_INTENT, SELECT_SCHEMA, SELECT_SCOPE,
    THREE_CARD_SPREAD_INTENT, THREE_CARD_SPREAD_INTENT_SCHEMA, THREE_CARD_SPREAD_SCOPE,
    ThreeCardSpreadIntentPayload,
};
pub use reading::{
    EXTERNAL_QUALIFICATION_ALGORITHM, EnrichmentQualification, Reading, ReadingEngine,
    ReadingError, Receipt, SelectionMode,
};
pub use servitor;
pub use servitors::{ServitorAccess, ServitorAccessError};
pub use session::{
    READING_SESSION_SCHEMA, REFLECTION_SCHEMA, ReadingPlacement, ReadingSession, Reflection,
    SessionError,
};
pub use spread::{
    SpreadError, THREE_CARD_SPREAD_SCHEMA, ThreeCardPlacement, ThreeCardPosition,
    ThreeCardRelation, ThreeCardRelationKind, ThreeCardSpread,
};
#[cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]
pub use sync::{
    CleromancySyncBatch, CleromancySyncError, CleromancySyncImport, CleromancySyncSelection,
    SYNC_BATCH_SCHEMA, export_sync_batch, import_sync_projection,
};
pub use tarot::{TarotPack, TarotQualification};

/// Stable fixture used by the A0 executable and integration receipts.
pub fn a0_fixture() -> (ContextSnapshot, Field) {
    let context = ContextSnapshot::new("A threshold", "cleromancy.fixture-context/v1")
        .with_fact("question", "What deserves attention at this threshold?")
        .with_fact("season", "late summer")
        .with_tags(["change", "structure", "reflection"]);
    let field = Field::new(
        "cleromancy.fixture-system/v1",
        CONTEXTUAL_WEIGHT_RULE,
        [
            Candidate::new(
                "threshold",
                "Attend to the threshold",
                "Notice what has already changed before deciding what should change next.",
            )
            .with_tags(["change", "reflection"])
            .with_base_weight(2),
            Candidate::new(
                "measure",
                "Measure the structure",
                "Name the constraint that is doing useful work and the one that has become habit.",
            )
            .with_tags(["structure", "reflection"])
            .with_base_weight(1),
            Candidate::new(
                "wander",
                "Permit an unplanned move",
                "Leave one part of the next step deliberately unspecified.",
            )
            .with_tags(["change", "play"])
            .with_base_weight(1),
        ],
    );
    (context, field)
}

/// A0's context plus explicit browsing terms used by the cross-process A1
/// Turnstone receipt. They remain ordinary disclosed context, not hidden
/// personalization.
pub fn a1_fixture() -> (ContextSnapshot, Field) {
    let (context, field) = a0_fixture();
    (
        context.with_fact(
            "recent_graph",
            "Field notes, a radio map, and a harmony map",
        ),
        field,
    )
}

/// A1's disclosed browsing context plus candidate tags which explicitly opt
/// into the A2 external-term qualifier.
pub fn a2_fixture() -> (ContextSnapshot, Field) {
    let (context, mut field) = a1_fixture();
    field.rules = EXTERNAL_TERM_WEIGHT_RULE.to_string();
    field.candidates[1]
        .tags
        .extend(["field", "harmony", "notes", "radio"].map(str::to_string));
    (context, field)
}
