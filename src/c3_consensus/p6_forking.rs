//! We saw in the previous chapter that blockchain communities sometimes opt to modify the
//! consensus rules from time to time. This process is knows as a fork. Here we implement
//! a higher-order consensus engine that allows such forks to be made.
//!
//! The consensus engine we implement here does not contain the specific consensus rules to
//! be enforced before or after the fork, but rather delegates to existing consensus engines
//! for that. Here we simply write the logic for detecting whether we are before or after the fork.

use std::marker::PhantomData;

use super::p1_pow::PoW;
use super::p3_poa::SimplePoa;
use super::p4_even_only::EvenOnly;
use super::{Consensus, ConsensusAuthority, Header};

/// A Higher-order consensus engine that represents a change from one set of consensus rules
/// (Before) to another set (After) at a specific block height
struct Forked<D, Before, After> {
	/// The first block height at which the new consensus rules apply
	fork_height: u64,
	digest: PhantomData<D>,
	engines: (Before, After),
}

impl<D, B, A> Consensus for Forked<D, B, A>
where
	D: Clone + core::fmt::Debug + Eq + PartialEq + std::hash::Hash,
	B: Consensus,
	A: Consensus,
	B::Digest: Into<D> + From<D>,
	A::Digest: Into<D> + From<D>,
{
	type Digest = D;

	fn validate(&self, parent_digest: &Self::Digest, header: &Header<Self::Digest>) -> bool {
		if header.height < self.fork_height {
			//convert
			let b_d_parent: B::Digest = parent_digest.clone().into();
			let header_new: Header<B::Digest> = Header {
				parent: header.parent,
				height: header.height,
				state_root: header.state_root,
				extrinsics_root: header.extrinsics_root,
				consensus_digest: header.consensus_digest.clone().into(),
			};
			//validate
			self.engines.0.validate(&b_d_parent, &header_new)
		} else {
			//convert
			let a_d_parent: A::Digest = parent_digest.clone().into();
			let header_new: Header<A::Digest> = Header {
				parent: header.parent,
				height: header.height,
				state_root: header.state_root,
				extrinsics_root: header.extrinsics_root,
				consensus_digest: header.consensus_digest.clone().into(),
			};
			//validate
			self.engines.1.validate(&a_d_parent, &header_new)
		}
	}

	fn seal(
		&self,
		parent_digest: &Self::Digest,
		partial_header: Header<()>,
	) -> Option<Header<Self::Digest>> {
		if partial_header.height < self.fork_height {
			let b_d_parent: B::Digest = parent_digest.clone().into();
			let seal = self.engines.0.seal(&b_d_parent, partial_header);
			match seal {
				None => None,
				Some(s) => {
					let header_new: Header<Self::Digest> = Header {
						parent: s.parent,
						height: s.height,
						state_root: s.state_root,
						extrinsics_root: s.extrinsics_root,
						consensus_digest: s.consensus_digest.clone().into(),
					};
					Some(header_new)
				},
			}
		} else {
			let a_d_parent: A::Digest = parent_digest.clone().into();
			let seal = self.engines.1.seal(&a_d_parent, partial_header);
			match seal {
				None => None,
				Some(s) => {
					let header_new: Header<Self::Digest> = Header {
						parent: s.parent,
						height: s.height,
						state_root: s.state_root,
						extrinsics_root: s.extrinsics_root,
						consensus_digest: s.consensus_digest.clone().into(),
					};
					Some(header_new)
				},
			}
		}
	}
}

/// Create a PoA consensus engine that changes authorities part way through the chain's history.
/// Given the initial authorities, the authorities after the fork, and the height at which the fork
/// occurs.
fn change_authorities(
	fork_height: u64,
	initial_authorities: Vec<ConsensusAuthority>,
	final_authorities: Vec<ConsensusAuthority>,
) -> impl Consensus {
	let poa_before = SimplePoa { authorities: initial_authorities };
	let poa_after = SimplePoa { authorities: final_authorities };
	Forked { 
		fork_height, 
		digest: PhantomData::<ConsensusAuthority>, 
		engines: (poa_before, poa_after) 
	}
}

/// Create a PoW consensus engine that changes the difficulty part way through the chain's history.
fn change_difficulty(
	fork_height: u64,
	initial_difficulty: u64,
	final_difficulty: u64,
) -> impl Consensus {
	let pow_before = PoW { threshold: initial_difficulty };
	let pow_after = PoW { threshold: final_difficulty };
	Forked { 
		fork_height, 
		digest: PhantomData::<u64>, 
		engines: (pow_before, pow_after) 
	}
}

/// Earlier in this chapter we implemented a consensus rule in which blocks are only considered
/// valid if they contain an even state root. Sometimes a chain will be launched with a more
/// traditional consensus like PoW or PoA and only introduce an additional requirement like even
/// state root after a particular height.
///
/// Create a consensus engine that introduces the even-only logic only after the given fork height.
/// Other than the evenness requirement, the consensus rules should not change at the fork. This
/// function should work with either PoW, PoA, or anything else as the underlying consensus engine.
fn even_after_given_height<Original: Consensus + Clone>(fork_height: u64, original: Original) -> impl Consensus {
	let cons_before = original;
	let cons_after = EvenOnly(cons_before.clone());
	Forked { 
		fork_height, 
		digest: PhantomData::<Original::Digest>, 
		engines: (cons_before, cons_after) 
	}
}

/// So far we have considered the simpler case where the consensus engines before and after the fork
/// use the same Digest type. Let us now turn our attention to the more general case where even the
/// digest type changes.
///
/// In order to implement a consensus change where even the Digest type changes, we will need an
/// enum that wraps the two individual digest types
#[derive(Hash, Debug, PartialEq, Eq, Clone, Copy)]
enum PowOrPoaDigest {
	Pow(u64),
	Poa(ConsensusAuthority),
}

impl From<u64> for PowOrPoaDigest {
	fn from(d: u64) -> Self {
		PowOrPoaDigest::Pow(d)
	}
}

impl From<ConsensusAuthority> for PowOrPoaDigest {
	fn from(d: ConsensusAuthority) -> Self {
		PowOrPoaDigest::Poa(d)
	}
}

impl From<PowOrPoaDigest> for ConsensusAuthority {
	fn from(d: PowOrPoaDigest) -> Self {
		match d {
			PowOrPoaDigest::Pow(_) => ConsensusAuthority::default(),
			PowOrPoaDigest::Poa(authority) => authority
		}
	}
}

impl From<PowOrPoaDigest> for u64 {
	fn from(d: PowOrPoaDigest) -> Self {
		match d {
			PowOrPoaDigest::Pow(thresh) => thresh,
			PowOrPoaDigest::Poa(_) => u64::MIN
		}
	}
}

/// In the spirit of Ethereum's recent switch from PoW to PoA, let us model a similar
/// switch in our consensus framework. It should go without saying that the real-world ethereum
/// handoff was considerably more complex than it may appear in our simplified example, although
/// the fundamentals are the same.
fn pow_to_poa(
	fork_height: u64,
	threshold: u64,
	authorities: Vec<ConsensusAuthority>,
) -> impl Consensus {
	let cons_before = PoW { threshold };
	let cons_after = SimplePoa { authorities };
	Forked { 
		fork_height, 
		digest: PhantomData::<PowOrPoaDigest>, 
		engines: (cons_before, cons_after) 
	}
}
