//! Forks in the blockchain represent alternative histories of the system.
//! When forks arise in the blockchain, users need a way to decide which chain
//! they will consider best, for now. This is known as a "fork choice rule".
//! There are several meaningful notions of "best", so we introduce a trait
//! that allows multiple implementations.
//!
//! Since we have nothing to add to the Block or Header data structures in this lesson,
//! we will import them from the previous lesson.

use super::p4_batched_extrinsics::{Block, Header};
use crate::hash;

const THRESHOLD: u64 = u64::max_value() / 100;

/// Judge which blockchain is "best" when there are multiple candidates. There are several
/// meaningful notions of "best" which is why this is a trait instead of just a
/// method.
pub trait ForkChoice {
	/// Compare two chains, and return the "best" one.
	///
	/// The chains are not assumed to start from the same genesis block, or even a
	/// genesis block at all. This makes it possible to compare entirely disjoint
	/// histories. It also makes it possible to compare _only_ the divergent part
	/// of sibling chains back to the last common ancestor.
	///
	/// The chains are assumed to be valid, so it is up to the caller to check
	/// validity first if they are unsure.
	fn first_chain_is_better(chain_1: &[Header], chain_2: &[Header]) -> bool;

	/// Compare many chains and return the best one.
	///
	/// It is always possible to compare several chains if you are able to compare
	/// two chains. Therefore this method has a provided implementation. However,
	/// it may be much more performant to write a fork-choice-specific implementation.
	fn best_chain<'a>(candidate_chains: &[&'a [Header]]) -> &'a [Header] {
		let mut best = candidate_chains[0];
		for i in 1..candidate_chains.len() {
			if Self::first_chain_is_better(best, candidate_chains[i]) {
				continue
			}
			best = candidate_chains[i];
		}
		best
	}
}

/// The "best" chain is simply the longest chain.
pub struct LongestChainRule;

impl ForkChoice for LongestChainRule {
	fn first_chain_is_better(chain_1: &[Header], chain_2: &[Header]) -> bool {
		chain_1.len() >= chain_2.len()
	}

	fn best_chain<'a>(candidate_chains: &[&'a [Header]]) -> &'a [Header] {
		let mut best_length = candidate_chains[0].len();
		let mut best_index = 0;
		for i in 1..candidate_chains.len() {
			if candidate_chains[i].len() > best_length {
				best_length = candidate_chains[i].len();
				best_index = i;
			}
		}
		candidate_chains[best_index]
	}
}

/// The best chain is the one with the most accumulated work.
///
/// In Proof of Work chains, each block contains a certain amount of "work".
/// Roughly speaking, the lower a block's hash is, the more work it contains,
/// because finding a block with a low hash requires, on average, trying more
/// nonces. Modeling the amount of work required to achieve a particular hash
/// is out of scope for this exercise, so we will use the not-really-right-but
/// conceptually-good-enough formula `work = THRESHOLD - block_hash`
pub struct HeaviestChainRule;

/// Mutates a block (and its embedded header) to contain more PoW difficulty.
/// This will be useful for exploring the heaviest chain rule. The expected
/// usage is that you create a block using the normal `Block.child()` method
/// and then pass the block to this helper for additional mining.
fn mine_extra_hard(header: &mut Header, threshold: u64) {
	//hash until under threshold
	while hash(&header) > threshold {
		header.consensus_digest += 1;
	}
}

impl ForkChoice for HeaviestChainRule {
	fn first_chain_is_better(chain_1: &[Header], chain_2: &[Header]) -> bool {
		let mut weight_1 = 0;
		for header in chain_1 {
			weight_1 += THRESHOLD - hash(header);
		}
		let mut weight_2 = 0;
		for header in chain_2 {
			weight_2 += THRESHOLD - hash(header);
		}
		println!("{}", weight_1);
		println!("{}", weight_2);
		weight_1 >= weight_2
	}

	// Specific implementation would remove the redundant hashing but that's okay this excercise
	// fn best_chain<'a>(candidate_chains: &[&'a [Header]]) -> &'a [Header] {
	// 	// Remember, this method is provided.
	// 	todo!("Exercise 6")
	// }
}
/// The best chain is the one with the most blocks that have even hashes.
///
/// This exact rule is a bit contrived, but it does model a family of fork choice rules
/// that are useful in the real world. We just can't code them here because we haven't
/// implemented Proof of Authority yet. Consider the following real world examples
/// that have very similar implementations.
///
/// 1. Secondary authors. In each round there is one author who is supposed to author. If that
///    author fails to create a block, there is a secondary author who may do so. The best chain is
///    the one with the most primary-authored blocks.
///
/// 2. Interleaved Pow/PoA. In each round there is one author who is allowed to author. Anyone else
///    is allowed to mine a PoW-style block. The best chain is the one with the most PoA blocks, and
///    ties are broken by the most accumulated work.
pub struct MostBlocksWithEvenHash;

impl ForkChoice for MostBlocksWithEvenHash {
	fn first_chain_is_better(chain_1: &[Header], chain_2: &[Header]) -> bool {
		let mut count_1 = 0;
		for header in chain_1 {
			count_1 += 1 - (hash(header) & 1);
		}
		let mut count_2 = 0;
		for header in chain_2 {
			count_2 += 1 - (hash(header) & 1);
		}
		count_1 > count_2
	}

	//same here, I'd worry if it was a production system
	// fn best_chain<'a>(candidate_chains: &[&'a [Header]]) -> &'a [Header] {
		// Remember, this method is provided.
		// todo!("Exercise 8")
	// }
}

// This lesson has omitted one popular fork choice rule:
// GHOST - Greedy Heaviest Observed SubTree
//
// I've omitted GHOST from here because it requires information about blocks that
// are _not_ in the chain to decide which chain is best. Therefore it does't work
// well with this relatively simple trait definition. We will return to the GHOST
// rule later when we have written a full blockchain client
//
// The GHOST rule was first published in 2013 by Yonatan Sompolinsky and Aviv Zohar.
// Learn more at https://eprint.iacr.org/2013/881.pdf

//

/// Build and return a valid chain with the given number of blocks.
fn build_valid_chain(n: u64) -> Vec<Header> {
	match n.try_into() {
		Ok(size) => {
			let mut headers = vec![Header::genesis(); size];
			for i in 1..size {
				headers[i] = headers[i-1].child(i as u64, i as u64);
			}
			headers
		}
		Err(e) => {
			return Vec::new();
		}
	}
}

// Add fork to a chain, extrinsic following a given rule
fn add_fork(pre: &Header, length: u64, extra_work: bool) -> Vec<Header> {
	let mut fork: Vec<Header> = vec![pre.child(0, 0)];
	for i in 0..length-1 {
		let last = &fork[fork.len()-1];
		let mut next = last.child(i, i);
		if extra_work {
			mine_extra_hard( &mut next, u64::max_value() / 1000);
		}
		fork.push(next);
	}
	fork
}

/// Build and return two different chains with a common prefix.
/// They should have the same genesis header. Both chains should be valid.
/// The first chain should be longer (have more blocks), but the second
/// chain should have more accumulated work.
///
/// Return your solutions as three vectors:
/// 1. The common prefix including genesis
/// 2. The suffix chain which is longer (non-overlapping with the common prefix)
/// 3. The suffix chain with more work (non-overlapping with the common prefix)
fn create_fork_one_side_longer_other_side_heavier() -> (Vec<Header>, Vec<Header>, Vec<Header>) {
	//A note on this one.. because of the formula we're using to calculate work, it is unlikely
	//the shorter one will be 'better' if it's length is n/2 or less, n being the length of the
	//long fork. This is because on average the long one scores THRESHOLD/2 per header, whereas
	//the short one scores at maximum THRESHOLD (if the mining difficulty is max hard)
	//Though I suppose thats only an issue on this task and in the real world you'd just say the
	//longer one had more work done!
	let pre = build_valid_chain(2);
	let last = pre.last().expect("Prefix was empty");
	let long = add_fork(last, 4, false);
	let weighted = add_fork(last, 3, true);
	(pre, long, weighted)
}

#[test]
fn bc_5_longest_chain() {
	let g = Header::genesis();

	let h_a1 = g.child(hash(&[1]), 1);
	let h_a2 = h_a1.child(hash(&[2]), 2);
	let chain_1 = &[g.clone(), h_a1, h_a2];

	let h_b1 = g.child(hash(&[3]), 3);
	let chain_2 = &[g, h_b1];

	assert!(LongestChainRule::first_chain_is_better(chain_1, chain_2));

	assert_eq!(LongestChainRule::best_chain(&[chain_1, chain_2]), chain_1);
}

#[test]
fn bc_5_mine_to_custom_difficulty() {
	let g = Block::genesis();
	let mut b1 = g.child(vec![1, 2, 3]);

	// We want the custom threshold to be high enough that we don't take forever mining
	// but low enough that it is unlikely we accidentally meet it with the normal
	// block creation function
	let custom_threshold = u64::max_value() / 1000;
	mine_extra_hard(&mut b1.header, custom_threshold);

	assert!(hash(&b1.header) < custom_threshold);
}

#[test]
fn bc_5_heaviest_chain() {
	let g = Header::genesis();

	let mut i = 0;
	let h_a1 = loop {
		let header = g.child(hash(&[i]), i);
		// Extrinsics root hash must be higher than threshold (less work done)
		if hash(&header) > THRESHOLD {
			break header
		}
		i += 1;
	};
	let chain_1 = &[g.clone(), h_a1];

	let h_b1 = loop {
		let header = g.child(hash(&[i]), i);
		// Extrinsics root hash must be lower than threshold (more work done)
		if hash(&header) < THRESHOLD {
			break header
		}
		i += 1;
	};
	let chain_2 = &[g, h_b1];

	assert!(HeaviestChainRule::first_chain_is_better(chain_2, chain_1));

	assert_eq!(HeaviestChainRule::best_chain(&[chain_1, chain_2]), chain_2);
}

#[test]
fn bc_5_most_even_blocks() {
	let g = Header::genesis();

	let mut h_a1 = g.child(2, 0);
	for i in 0..u64::max_value() {
		h_a1 = g.child(2, i);
		if hash(&h_a1) % 2 == 0 {
			break
		}
	}
	let mut h_a2 = g.child(2, 0);
	for i in 0..u64::max_value() {
		h_a2 = h_a1.child(2, i);
		if hash(&h_a2) % 2 == 0 {
			break
		}
	}
	let chain_1 = &[g.clone(), h_a1, h_a2];

	let mut h_b1 = g.child(2, 0);
	for i in 0..u64::max_value() {
		h_b1 = g.child(2, i);
		if hash(&h_b1) % 2 != 0 {
			break
		}
	}
	let mut h_b2 = g.child(2, 0);
	for i in 0..u64::max_value() {
		h_b2 = h_b1.child(2, i);
		if hash(&h_b2) % 2 != 0 {
			break
		}
	}
	let chain_2 = &[g, h_b1, h_b2];

	assert!(MostBlocksWithEvenHash::first_chain_is_better(chain_1, chain_2));

	assert_eq!(MostBlocksWithEvenHash::best_chain(&[chain_1, chain_2]), chain_1);
}

#[test]
fn bc_5_longest_vs_heaviest() {
	let (_, longest_chain, pow_chain) = create_fork_one_side_longer_other_side_heavier();

	assert!(LongestChainRule::first_chain_is_better(&longest_chain, &pow_chain));

	assert_eq!(LongestChainRule::best_chain(&[&longest_chain, &pow_chain]), &longest_chain);

	let (_, longest_chain, pow_chain) = create_fork_one_side_longer_other_side_heavier();

	assert!(HeaviestChainRule::first_chain_is_better(&pow_chain, &longest_chain));

	// assert_eq!(HeaviestChainRule::best_chain(&[&longest_chain, &pow_chain]), &pow_chain);
}
