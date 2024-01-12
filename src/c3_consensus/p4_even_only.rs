//! In the previous chapter, we considered a hypothetical scenario where blocks must contain an even
//! state root in order to be valid. Now we will express that logic here as a higher-order consensus
//! engine. It is higher- order because it will wrap an inner consensus engine, such as PoW or PoA
//! and work in either case.

use super::{Consensus, Header, p1_pow::PoW};
use crate::hash;

/// A Consensus engine that wraps another consensus engine. This engine enforces the requirement
/// that a block must have an even state root in order to be valid

/// A Consensus engine that requires the state root to be even for the header to be valid.
/// Wraps an inner consensus engine whose rules will also be enforced.
struct EvenOnly<Inner: Consensus>(Inner);

impl<Inner: Consensus> Consensus for EvenOnly<Inner> {
	type Digest = Inner::Digest;

	fn validate(&self, parent_digest: &Self::Digest, header: &Header<Self::Digest>) -> bool {
		let inner_valid = self.0.validate(parent_digest, header);
		let valid = header.state_root & 1 != 1;
		inner_valid && valid
	}

	fn seal(
		&self,
		parent_digest: &Self::Digest,
		partial_header: Header<()>,
	) -> Option<Header<Self::Digest>> {
		let header = self.0.seal(parent_digest, partial_header).unwrap();
		if self.validate(parent_digest, &header) {
			Some(header);
		}
		None
	}
}

/// Using the moderate difficulty PoW algorithm you created in section 1 of this chapter as the
/// inner engine, create a PoW chain that is valid according to the inner consensus engine, but is
/// not valid according to this engine because the state roots are not all even.
fn almost_valid_but_not_all_even() -> Vec<Header<u64>> {
	//engines
	let inner_engine = PoW{threshold: u64::MAX/100};
	let engine = EvenOnly(inner_engine);
	
	//genesis
	let mut headers: Vec<Header<u64>> = vec![];
	let mut partial: Header<()> = Header {
		parent: u64::MIN,
		height: 0,
		state_root: u64::MIN,
		extrinsics_root: u64::MIN,
		consensus_digest: (),
	};
	
	//add until state_root is odd, assume exts are just +1 and state just goes up by 1
	while let Some(h) = engine.seal(&u64::MIN, partial) {
		headers.push(h.clone());
		partial = Header {
			parent: hash(&h),
			height: h.height+1,
			state_root: hash(&(h.height+1)),
			extrinsics_root: hash(&vec![1]),
			consensus_digest: (),
		};
	}
	headers
}
