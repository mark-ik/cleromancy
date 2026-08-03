// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};

use crate::context::canonical_digest;
use crate::{CONTEXTUAL_WEIGHT_RULE, Candidate, Field, UNIFORM_RULE};

pub const MAJOR_ARCANA_PACK_SCHEMA: &str = "cleromancy.tarot-pack/v1";
pub const RWS_MAJOR_ARCANA_ID: &str = "cleromancy.tarot/rws-major-reflective-v1";

/// The user-visible way a Tarot field is qualified. Uniform ignores context;
/// contextual gives each matching context tag one additional base-weight
/// share. Both choices remain explicit in the field and reading receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TarotQualification {
    Uniform,
    Contextual,
}

impl TarotQualification {
    pub fn rule(self) -> &'static str {
        match self {
            Self::Uniform => UNIFORM_RULE,
            Self::Contextual => CONTEXTUAL_WEIGHT_RULE,
        }
    }
}

/// A bounded built-in data pack, not a general catalog or plug-in contract.
/// It uses the familiar Rider-Waite-Smith major ordering and original
/// Cleromancy prompts. It deliberately declares neither reversals nor
/// astrology correspondences.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TarotPack {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub ordering: String,
    pub content: String,
    pub candidates: Vec<Candidate>,
}

impl TarotPack {
    pub fn rws_major_arcana() -> Self {
        Self {
            schema: MAJOR_ARCANA_PACK_SCHEMA.to_string(),
            id: RWS_MAJOR_ARCANA_ID.to_string(),
            title: "Major Arcana: reflective prompts".to_string(),
            ordering: "Rider-Waite-Smith order; Strength VIII and Justice XI".to_string(),
            content: "Traditional card titles with original upright reflective prompts".to_string(),
            candidates: major_arcana(),
        }
    }

    pub fn field(&self, qualification: TarotQualification) -> Field {
        Field::new(
            self.id.clone(),
            qualification.rule(),
            self.candidates.clone(),
        )
    }

    pub fn digest(&self) -> String {
        canonical_digest(self)
    }
}

fn major_arcana() -> Vec<Candidate> {
    vec![
        card(
            0,
            "fool",
            "The Fool",
            "What beginning asks for curiosity before certainty, and what risk can remain bounded?",
            ["beginning", "openness", "risk"],
        ),
        card(
            1,
            "magician",
            "The Magician",
            "Which available skill or tool becomes effective when you give it undivided attention?",
            ["agency", "attention", "skill"],
        ),
        card(
            2,
            "high-priestess",
            "The High Priestess",
            "What is not yet ready to be explained, and what can be learned by listening longer?",
            ["ambiguity", "intuition", "silence"],
        ),
        card(
            3,
            "empress",
            "The Empress",
            "What needs material care, patience, or room to grow?",
            ["abundance", "embodiment", "nurture"],
        ),
        card(
            4,
            "emperor",
            "The Emperor",
            "Which boundary or structure would make action safer and clearer?",
            ["authority", "boundary", "structure"],
        ),
        card(
            5,
            "hierophant",
            "The Hierophant",
            "Which inherited practice is teaching you, and which part deserves examination?",
            ["institution", "teaching", "tradition"],
        ),
        card(
            6,
            "lovers",
            "The Lovers",
            "What choice would align relationship, desire, and stated values?",
            ["choice", "relationship", "values"],
        ),
        card(
            7,
            "chariot",
            "The Chariot",
            "What competing impulses need one direction before momentum becomes useful?",
            ["direction", "discipline", "momentum"],
        ),
        card(
            8,
            "strength",
            "Strength",
            "Where would patient courage accomplish more than force?",
            ["courage", "patience", "restraint"],
        ),
        card(
            9,
            "hermit",
            "The Hermit",
            "What becomes audible when you step back from other people's urgency?",
            ["guidance", "inquiry", "solitude"],
        ),
        card(
            10,
            "wheel-of-fortune",
            "Wheel of Fortune",
            "Which change is already in motion, and where can you respond without pretending to control it?",
            ["chance", "change", "cycle"],
        ),
        card(
            11,
            "justice",
            "Justice",
            "What consequence, obligation, or imbalance needs an exact accounting?",
            ["accountability", "balance", "consequence"],
        ),
        card(
            12,
            "hanged-man",
            "The Hanged Man",
            "What changes when you stop forcing movement and examine the situation from its underside?",
            ["perspective", "surrender", "suspension"],
        ),
        card(
            13,
            "death",
            "Death",
            "What ending must be acknowledged before transformation becomes more than a slogan?",
            ["ending", "release", "transformation"],
        ),
        card(
            14,
            "temperance",
            "Temperance",
            "Which elements need proportion, exchange, and repeated adjustment rather than a dramatic solution?",
            ["exchange", "integration", "moderation"],
        ),
        card(
            15,
            "devil",
            "The Devil",
            "Which attachment gains power by remaining unnamed, and what choice would loosen it?",
            ["appetite", "attachment", "constraint"],
        ),
        card(
            16,
            "tower",
            "The Tower",
            "What unstable structure is being exposed, and what truth survives its collapse?",
            ["instability", "revelation", "rupture"],
        ),
        card(
            17,
            "star",
            "The Star",
            "What small source of orientation can you trust while repair is still incomplete?",
            ["hope", "orientation", "renewal"],
        ),
        card(
            18,
            "moon",
            "The Moon",
            "Which fear, image, or ambiguity needs observation before it is treated as fact?",
            ["fear", "imagination", "uncertainty"],
        ),
        card(
            19,
            "sun",
            "The Sun",
            "What becomes simpler when it is brought fully into view and shared?",
            ["clarity", "recognition", "vitality"],
        ),
        card(
            20,
            "judgement",
            "Judgement",
            "What prior decision is asking to be heard, revised, or answered now?",
            ["calling", "reckoning", "revision"],
        ),
        card(
            21,
            "world",
            "The World",
            "What is actually complete, and what new threshold appears when you name that completion?",
            ["completion", "passage", "wholeness"],
        ),
    ]
}

fn card(number: u8, id: &str, title: &str, interpretation: &str, tags: [&str; 3]) -> Candidate {
    Candidate::new(format!("major-{number:02}-{id}"), title, interpretation).with_tags(tags)
}
