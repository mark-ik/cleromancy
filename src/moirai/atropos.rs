// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::context::canonical_digest;
use crate::field::{Candidate, Field};
use crate::reading::{Reading, Receipt};

/// Seal a selected candidate and its provenance into an immutable reading.
pub(crate) fn seal(field: &Field, candidate: &Candidate, receipt: Receipt) -> Reading {
    let id = canonical_digest(&(&field.system, &candidate.id, &receipt));
    let schema = if receipt.enrichment.is_some() {
        "cleromancy.reading/v2"
    } else {
        "cleromancy.reading/v1"
    };
    Reading {
        schema: schema.to_string(),
        id,
        system: field.system.clone(),
        candidate_id: candidate.id.clone(),
        title: candidate.title.clone(),
        interpretation: candidate.interpretation.clone(),
        receipt,
    }
}
