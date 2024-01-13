//! PoW and PoA each have their own set of strengths and weaknesses. Many chains are happy to choose
//! one of them. But other chains would like consensus properties that fall in between. To achieve
//! this we could consider interleaving PoW blocks with PoA blocks. Some very early designs of
//! Ethereum considered this approach as a way to transition away from PoW.

use super::{Consensus, ConsensusAuthority, Header};
use crate::hash;

//the digest for an alternating consensus engine
#[derive(Hash, Debug, PartialEq, Eq, Clone, Copy)]
struct AltDigest {
	nonce: u64,
	auth: ConsensusAuthority,
}

/// A Consensus engine that alternates back and forth between PoW and PoA sealed blocks.
/// Will just do simple dictator for PoA
struct AlternatingPowPoa {
	threshold: u64,
	dictator: ConsensusAuthority,
}

impl Consensus for AlternatingPowPoa {
	type Digest = AltDigest;

	fn validate(&self, _: &Self::Digest, header: &Header<Self::Digest>) -> bool {
		if header.height % 2 == 1 {
			//PoW
			hash(&header) < self.threshold
		} else {
			//PoA
			header.consensus_digest.auth == self.dictator
		}
	}

	fn seal(&self, _: &Self::Digest, partial_header: Header<()>) -> Option<Header<Self::Digest>> {
		if partial_header.height % 2 == 0 {
			//PoW
			let mut header: Header<Self::Digest> = partial_header.convert_to_digest(
                AltDigest { nonce: u64::MIN, auth: ConsensusAuthority::default() }
            );
			while hash(&header) < self.threshold {
				header.consensus_digest.nonce += 1;
			}
			Some(header)
		} else {
			//PoA
			Some(partial_header.convert_to_digest(AltDigest { nonce: u64::MIN, auth: self.dictator }))
		}
	}
}
