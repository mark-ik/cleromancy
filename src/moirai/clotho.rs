// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use rand_core::{OsRng, RngCore};

use crate::reading::ReadingError;

/// Replaceable entropy seam. Production uses [`OsEntropy`]; tests inject exact
/// words so rejection, selection, and replay can be proved independently.
pub trait EntropySource {
    fn next_u64(&mut self) -> Result<u64, ReadingError>;
}

/// Operating-system cryptographic randomness through `rand_core::OsRng`.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsEntropy;

impl EntropySource for OsEntropy {
    fn next_u64(&mut self) -> Result<u64, ReadingError> {
        let mut bytes = [0u8; 8];
        OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|error| ReadingError::Entropy(error.to_string()))?;
        Ok(u64::from_le_bytes(bytes))
    }
}

/// A bounded random draw plus a separate event nonce. `sample` is already in
/// `[0, upper)`, so a receipt can replay weighted selection without claiming
/// to reconstruct or expose the operating system's entropy state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draw {
    pub sample: u64,
    pub event_nonce: String,
}

/// Rejection sampling over the full `u64` output space. Modulo is applied only
/// after the accepted range has a size divisible by `upper`.
pub fn draw_below(entropy: &mut impl EntropySource, upper: u64) -> Result<Draw, ReadingError> {
    if upper == 0 {
        return Err(ReadingError::EmptyWeight);
    }
    let limit = u64::MAX - (u64::MAX % upper);
    let sample = loop {
        let raw = entropy.next_u64()?;
        if raw < limit {
            break raw % upper;
        }
    };
    let high = entropy.next_u64()?;
    let low = entropy.next_u64()?;
    Ok(Draw {
        sample,
        event_nonce: format!("{high:016x}{low:016x}"),
    })
}
