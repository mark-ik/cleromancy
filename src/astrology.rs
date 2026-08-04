// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::context::canonical_digest;

/// A chart is a disclosed result from an ephemeris adapter. Cleromancy does
/// not claim to calculate planetary positions in this module.
pub const ASTROLOGY_CHART_SCHEMA: &str = "cleromancy.astrology-chart/v1";
/// Structured sign and aspect derivation from the supplied chart positions.
pub const ASTROLOGY_FACTS_SCHEMA: &str = "cleromancy.astrology-facts/v1";
pub const ASTROLOGY_FACTS_ALGORITHM: &str =
    "cleromancy.astrology/signs-and-aspects-from-positions/v1";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AstrologyError {
    #[error("astrology {0} is empty")]
    Empty(&'static str),
    #[error("astrology body is empty or duplicated: {0}")]
    InvalidBody(String),
    #[error("astrology longitude must be in 0..360000 millidegrees: {0}")]
    InvalidLongitude(u32),
    #[error("astrology latitude must be in -90000..90000 millidegrees: {0}")]
    InvalidLatitude(i32),
    #[error("astrology latitude and longitude must both be present or absent")]
    IncompleteLocation,
    #[error("astrology latitude must be in -90000000..90000000 microdegrees: {0}")]
    InvalidLatitudeMicrodegrees(i32),
    #[error("astrology longitude must be in -180000000..180000000 microdegrees: {0}")]
    InvalidLongitudeMicrodegrees(i32),
    #[error("astrology aspect orb must be at most 180000 millidegrees: {0}")]
    InvalidOrb(u32),
    #[error("astrology chart does not match its declared schema")]
    InvalidChartSchema,
    #[error("astrology facts do not match their declared inputs: {0}")]
    FactsMismatch(&'static str),
}

/// The instant and optional location supplied to the chart calculator. The
/// timestamp is retained as an adapter-provided UTC string rather than parsed
/// here, so this crate does not silently normalize or invent a timezone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AstrologyMoment {
    pub instant_utc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latitude_microdegrees: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub longitude_microdegrees: Option<i32>,
}

impl AstrologyMoment {
    pub fn global(instant_utc: impl Into<String>) -> Self {
        Self {
            instant_utc: instant_utc.into(),
            latitude_microdegrees: None,
            longitude_microdegrees: None,
        }
    }

    pub fn at(
        instant_utc: impl Into<String>,
        latitude_microdegrees: i32,
        longitude_microdegrees: i32,
    ) -> Self {
        Self {
            instant_utc: instant_utc.into(),
            latitude_microdegrees: Some(latitude_microdegrees),
            longitude_microdegrees: Some(longitude_microdegrees),
        }
    }

    fn validate(&self) -> Result<(), AstrologyError> {
        if self.instant_utc.trim().is_empty() {
            return Err(AstrologyError::Empty("instant_utc"));
        }
        match (self.latitude_microdegrees, self.longitude_microdegrees) {
            (None, None) => Ok(()),
            (Some(latitude), Some(longitude)) => {
                if !(-90_000_000..=90_000_000).contains(&latitude) {
                    return Err(AstrologyError::InvalidLatitudeMicrodegrees(latitude));
                }
                if !(-180_000_000..=180_000_000).contains(&longitude) {
                    return Err(AstrologyError::InvalidLongitudeMicrodegrees(longitude));
                }
                Ok(())
            }
            _ => Err(AstrologyError::IncompleteLocation),
        }
    }
}

/// One body position copied from the named calculation source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AstrologyPosition {
    pub body: String,
    pub longitude_millidegrees: u32,
    pub latitude_millidegrees: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrograde: Option<bool>,
}

impl AstrologyPosition {
    pub fn new(
        body: impl Into<String>,
        longitude_millidegrees: u32,
        latitude_millidegrees: i32,
    ) -> Self {
        Self {
            body: body.into(),
            longitude_millidegrees,
            latitude_millidegrees,
            retrograde: None,
        }
    }

    pub fn with_retrograde(mut self, retrograde: bool) -> Self {
        self.retrograde = Some(retrograde);
        self
    }

    fn validate(&self) -> Result<(), AstrologyError> {
        if self.body.trim().is_empty() {
            return Err(AstrologyError::InvalidBody(self.body.clone()));
        }
        if self.longitude_millidegrees >= 360_000 {
            return Err(AstrologyError::InvalidLongitude(
                self.longitude_millidegrees,
            ));
        }
        if !(-90_000..=90_000).contains(&self.latitude_millidegrees) {
            return Err(AstrologyError::InvalidLatitude(self.latitude_millidegrees));
        }
        Ok(())
    }
}

/// A source-qualified set of positions. The source metadata is part of the
/// digest and must be retained when this value is persisted or synced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AstrologyChart {
    pub schema: String,
    pub algorithm: String,
    pub engine: String,
    pub ephemeris: String,
    pub moment: AstrologyMoment,
    pub positions: Vec<AstrologyPosition>,
}

impl AstrologyChart {
    pub fn new(
        algorithm: impl Into<String>,
        engine: impl Into<String>,
        ephemeris: impl Into<String>,
        moment: AstrologyMoment,
        positions: impl IntoIterator<Item = AstrologyPosition>,
    ) -> Result<Self, AstrologyError> {
        let mut chart = Self {
            schema: ASTROLOGY_CHART_SCHEMA.to_string(),
            algorithm: algorithm.into(),
            engine: engine.into(),
            ephemeris: ephemeris.into(),
            moment,
            positions: positions.into_iter().collect(),
        };
        chart
            .positions
            .sort_by(|left, right| left.body.cmp(&right.body));
        chart.validate()?;
        Ok(chart)
    }

    pub fn validate(&self) -> Result<(), AstrologyError> {
        if self.schema != ASTROLOGY_CHART_SCHEMA {
            return Err(AstrologyError::InvalidChartSchema);
        }
        for (name, value) in [
            ("algorithm", self.algorithm.as_str()),
            ("engine", self.engine.as_str()),
            ("ephemeris", self.ephemeris.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(AstrologyError::Empty(name));
            }
        }
        self.moment.validate()?;
        if self.positions.is_empty() {
            return Err(AstrologyError::Empty("positions"));
        }
        let mut bodies = BTreeSet::new();
        for position in &self.positions {
            position.validate()?;
            if !bodies.insert(position.body.as_str()) {
                return Err(AstrologyError::InvalidBody(position.body.clone()));
            }
        }
        Ok(())
    }

    pub fn digest(&self) -> String {
        canonical_digest(self)
    }

    pub fn position(&self, body: &str) -> Option<&AstrologyPosition> {
        self.positions.iter().find(|position| position.body == body)
    }

    /// Derive only structured astronomical descriptors. This does not create
    /// a reading, a personality claim, or a natural-language interpretation.
    pub fn facts(&self, orb_millidegrees: u32) -> Result<AstrologyFacts, AstrologyError> {
        self.validate()?;
        if orb_millidegrees > 180_000 {
            return Err(AstrologyError::InvalidOrb(orb_millidegrees));
        }
        let placements = self
            .positions
            .iter()
            .map(AstrologyPlacement::from_position)
            .collect();
        let aspects = derive_aspects(&self.positions, orb_millidegrees);
        Ok(AstrologyFacts {
            schema: ASTROLOGY_FACTS_SCHEMA.to_string(),
            algorithm: ASTROLOGY_FACTS_ALGORITHM.to_string(),
            chart_digest: self.digest(),
            orb_millidegrees,
            placements,
            aspects,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZodiacSign {
    Aries,
    Taurus,
    Gemini,
    Cancer,
    Leo,
    Virgo,
    Libra,
    Scorpio,
    Sagittarius,
    Capricorn,
    Aquarius,
    Pisces,
}

impl ZodiacSign {
    fn from_longitude(longitude_millidegrees: u32) -> Self {
        const SIGNS: [ZodiacSign; 12] = [
            ZodiacSign::Aries,
            ZodiacSign::Taurus,
            ZodiacSign::Gemini,
            ZodiacSign::Cancer,
            ZodiacSign::Leo,
            ZodiacSign::Virgo,
            ZodiacSign::Libra,
            ZodiacSign::Scorpio,
            ZodiacSign::Sagittarius,
            ZodiacSign::Capricorn,
            ZodiacSign::Aquarius,
            ZodiacSign::Pisces,
        ];
        SIGNS[(longitude_millidegrees / 30_000) as usize]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AstrologyPlacement {
    pub body: String,
    pub sign: ZodiacSign,
    pub degree_millidegrees: u32,
    pub latitude_millidegrees: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrograde: Option<bool>,
}

impl AstrologyPlacement {
    fn from_position(position: &AstrologyPosition) -> Self {
        Self {
            body: position.body.clone(),
            sign: ZodiacSign::from_longitude(position.longitude_millidegrees),
            degree_millidegrees: position.longitude_millidegrees % 30_000,
            latitude_millidegrees: position.latitude_millidegrees,
            retrograde: position.retrograde,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AspectKind {
    Conjunction,
    Sextile,
    Square,
    Trine,
    Opposition,
}

impl AspectKind {
    fn exact_millidegrees(self) -> u32 {
        match self {
            Self::Conjunction => 0,
            Self::Sextile => 60_000,
            Self::Square => 90_000,
            Self::Trine => 120_000,
            Self::Opposition => 180_000,
        }
    }

    fn ordered() -> [Self; 5] {
        [
            Self::Conjunction,
            Self::Sextile,
            Self::Square,
            Self::Trine,
            Self::Opposition,
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AstrologyAspect {
    pub first: String,
    pub second: String,
    pub kind: AspectKind,
    pub exact_millidegrees: u32,
    pub separation_millidegrees: u32,
    pub orb_millidegrees: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AstrologyFacts {
    pub schema: String,
    pub algorithm: String,
    pub chart_digest: String,
    pub orb_millidegrees: u32,
    pub placements: Vec<AstrologyPlacement>,
    pub aspects: Vec<AstrologyAspect>,
}

impl AstrologyFacts {
    pub fn verify(&self, chart: &AstrologyChart) -> Result<(), AstrologyError> {
        if self.schema != ASTROLOGY_FACTS_SCHEMA {
            return Err(AstrologyError::FactsMismatch("schema"));
        }
        if self.algorithm != ASTROLOGY_FACTS_ALGORITHM {
            return Err(AstrologyError::FactsMismatch("algorithm"));
        }
        if self.chart_digest != chart.digest() {
            return Err(AstrologyError::FactsMismatch("chart digest"));
        }
        let rebuilt = chart.facts(self.orb_millidegrees)?;
        if rebuilt.placements != self.placements {
            return Err(AstrologyError::FactsMismatch("placements"));
        }
        if rebuilt.aspects != self.aspects {
            return Err(AstrologyError::FactsMismatch("aspects"));
        }
        Ok(())
    }

    pub fn digest(&self) -> String {
        canonical_digest(self)
    }
}

fn derive_aspects(positions: &[AstrologyPosition], orb_millidegrees: u32) -> Vec<AstrologyAspect> {
    let mut aspects = Vec::new();
    for (index, first) in positions.iter().enumerate() {
        for second in positions.iter().skip(index + 1) {
            let direct = first
                .longitude_millidegrees
                .abs_diff(second.longitude_millidegrees);
            let separation = direct.min(360_000 - direct);
            let Some((kind, orb)) = AspectKind::ordered()
                .into_iter()
                .map(|kind| {
                    let exact = kind.exact_millidegrees();
                    (kind, separation.abs_diff(exact))
                })
                .filter(|(_, orb)| *orb <= orb_millidegrees)
                .min_by_key(|(kind, orb)| (*orb, kind.exact_millidegrees()))
            else {
                continue;
            };
            aspects.push(AstrologyAspect {
                first: first.body.clone(),
                second: second.body.clone(),
                kind,
                exact_millidegrees: kind.exact_millidegrees(),
                separation_millidegrees: separation,
                orb_millidegrees: orb,
            });
        }
    }
    aspects
}
