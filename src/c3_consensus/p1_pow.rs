//! Proof of Work provides security to the blockchain by requiring block authors
//! to expend a real-world scarce resource, namely energy, in order to author a valid block.
//!
//! This is the same logic we implemented previously. Here we re-implement it in the
//! generic consensus framework that we will use throughout the rest of the chapter.

use std::num::ParseIntError;

use super::{Consensus, Header};
use crate::hash;

/// A Proof of Work consensus engine. This is the same consensus logic that we
/// implemented in the previous chapter. Here we simply re-implement it in the
/// consensus framework that will be used throughout this chapter.
pub struct PoW {
	pub threshold: u64,
}

impl Consensus for PoW {
	type Digest = u64;

	/// Check that the provided header's hash is below the required threshold.
	/// This does not rely on the parent digest at all.
	fn validate(&self, _: &Self::Digest, header: &Header<Self::Digest>) -> bool {
		header.consensus_digest < self.threshold
	}

	/// Mine a new PoW seal for the partial header provided.
	/// This does not rely on the parent digest at all.
	fn seal(&self, _: &Self::Digest, partial_header: Header<()>) -> Option<Header<Self::Digest>> {
		let mut header: Header<Self::Digest> = partial_header.convert_to_digest(u64::MIN);
		let mut hashed = hash(&header);
		while hashed >= self.threshold {
			header.consensus_digest += 1;
			hashed = hash(&header);
		}
		Some(header)
	}
}

/// Create a PoW consensus engine that has a difficulty threshold such that roughly 1 in 100 blocks
/// with randomly drawn nonces will be valid. That is: the threshold should be u64::max_value() /
/// 100.
pub fn moderate_difficulty_pow() -> impl Consensus {
	PoW {
		threshold: u64::max_value() / 100
	}
}
