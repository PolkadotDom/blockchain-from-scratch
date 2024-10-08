//! Until now, each block has contained just a single extrinsic. Really we would prefer to batch
//! them. Now, we stop relying solely on headers, and instead, create complete blocks.

use crate::hash;
type Hash = u64;

const THRESHOLD: u64 = u64::max_value() / 100;

/// The header no longer contains an extrinsic directly. Rather a vector of extrinsics will be
/// stored in the block body. We are still storing the state in the header for now. This will change
/// in an upcoming lesson as well.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Header {
	parent: Hash,
	height: u64,
	// We now switch from storing an extrinsic directly, to storing an extrinsic root.
	// This is basically a concise cryptographic commitment to the complete list of extrinsics.
	// For example, a hash or a Merkle root.
	extrinsics_root: Hash,
	state: u64,
	pub consensus_digest: u64,
}

// Methods for creating and verifying headers.
//
// With the extrinsics no longer stored in the header, we can no longer do
// "on-chain" execution with just headers. That means that this code actually
// gets simpler in many ways. All the old execution logic, plus some new batching
// logic moves to the block level now.
impl Header {
	/// Returns a new valid genesis header.
	pub fn genesis() -> Self {
		Header {
			parent: u64::MIN,
			height: u64::MIN,
			extrinsics_root: u64::MIN,
			state: 0,
			consensus_digest: 0,
		}
	}

	/// Create and return a valid child header.
	/// Without the extrinsics themselves, we cannot calculate the final state
	/// so that information is passed in.
	pub fn child(&self, extrinsics_root: Hash, state: u64) -> Self {
		let mut h = Header {
			parent: hash(self),
			height: self.height + 1,
			extrinsics_root,
			state,
			consensus_digest: u64::MIN,
		};
		//hash until under threshold
		while hash(&h) > THRESHOLD {
			h.consensus_digest += 1;
		}
		h
	}

	/// Verify a single child header.
	///
	/// This is a slightly different interface from the previous units. Rather
	/// than verify an entire sub-chain, this function checks a single header.
	/// This is useful because checking the header can now be thought of as a
	/// subtask of checking an entire block. So it doesn't make sense to check
	/// the entire header chain at once if the chain may be invalid at the second block.
	fn verify_child(&self, child: &Header) -> bool {
		child.parent == hash(self) && child.height == self.height + 1
	}

	/// Verify that all the given headers form a valid chain from this header to the tip.
	///
	/// We can now trivially write the old verification function in terms of the new one.
	/// Extra street cred if you can write it
	///  - with a loop
	///  - with head recursion
	///  - with tail recursion
	fn verify_sub_chain(&self, chain: &[Header]) -> bool {
		for i in 1..chain.len() {
			if !chain[i - 1].verify_child(&chain[i]) {
				return false;
			}
		}
		true
	}
}

/// A complete Block is a header and the extrinsics.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Block {
	pub(crate) header: Header,
	pub(crate) body: Vec<u64>,
}

// Methods for creating and verifying blocks.
//
// These methods are analogous to the methods on the headers. All of the
// transaction execution logic is now handled at the block level because
// the transactions are no longer available at the Header level.
impl Block {
	/// Returns a new valid genesis block. By convention this block has no extrinsics.
	pub fn genesis() -> Self {
		Block { header: Header::genesis(), body: vec![] }
	}

	//execute the extrinsics on the state
	fn execute_exts(mut prev_state: u64, exts: &Vec<u64>) -> u64 {
		for ext in exts {
			prev_state += ext;
		}
		prev_state
	}

	/// Create and return a valid child block.
	/// The extrinsics are batched now, so we need to execute each of them.
	pub fn child(&self, extrinsics: Vec<u64>) -> Self {
		let new_state = Self::execute_exts(self.header.state, &extrinsics);
		Block { header: self.header.child(hash(&extrinsics), new_state), body: extrinsics }
	}

	//verify parent and child block
	fn verify_block_child(parent: &Block, child: &Block) -> bool {
		let header_cond = parent.header.verify_child(&child.header);
		let body_cond = child.header.state == Self::execute_exts(parent.header.state, &child.body);
		if !(header_cond && body_cond) {
			return false;
		}
		true
	}

	/// Verify that all the given blocks form a valid chain from this block to the tip.
	///
	/// We need to verify the headers as well as execute all transactions and check the final state.
	pub fn verify_sub_chain(&self, chain: &[Block]) -> bool {
		//first check
		if !Self::verify_block_child(self, &chain[0]) {
			return false;
		}
		//rest of chain
		for i in 0..chain.len() - 1 {
			if !Self::verify_block_child(&chain[i], &chain[i+1]) {
				return false;
			}
		}
		true
	}
}

/// Create an invalid child block of the given block. Although the child block is invalid,
/// the header should be valid.
///
/// Now that extrinsics are separate from headers, the logic for checking headers does
/// not include actual transaction execution. That means it is possible for a header to be
/// valid, but the block containing that header to be invalid.
///
/// Notice that you do not need the entire parent block to do this. You only need the header.
fn build_invalid_child_block_with_valid_header(parent: &Header) -> Block {
	Block { 
		header: parent.child(u64::MIN, u64::MAX),
		body: vec![] 
	}
}

#[test]
fn bc_4_genesis_header() {
	let g = Header::genesis();
	assert_eq!(g.height, 0);
	assert_eq!(g.parent, 0);
	assert_eq!(g.extrinsics_root, 0);
	assert_eq!(g.state, 0);
}

#[test]
fn bc_4_genesis_block() {
	let gh = Header::genesis();
	let gb = Block::genesis();

	assert_eq!(gb.header, gh);
	assert!(gb.body.is_empty());
}

#[test]
fn bc_4_child_block_empty() {
	let b0 = Block::genesis();
	let b1 = b0.child(vec![]);

	assert_eq!(b1.header.height, 1);
	assert_eq!(b1.header.parent, hash(&b0.header));
	assert_eq!(b1, Block { header: b1.header.clone(), body: vec![] });
}

#[test]
fn bc_4_child_block() {
	let b0 = Block::genesis();
	let b1 = b0.child(vec![1, 2, 3, 4, 5]);

	assert_eq!(b1.header.height, 1);
	assert_eq!(b1.header.parent, hash(&b0.header));
	assert_eq!(b1, Block { header: b1.header.clone(), body: vec![1, 2, 3, 4, 5] });
}

#[test]
fn bc_4_child_header() {
	let g = Header::genesis();
	let h1 = g.child(hash(&[1, 2, 3]), 6);

	assert_eq!(h1.height, 1);
	assert_eq!(h1.parent, hash(&g));
	assert_eq!(h1.extrinsics_root, hash(&[1, 2, 3]));
	assert_eq!(h1.state, 6);

	let h2 = h1.child(hash(&[10, 20]), 36);

	assert_eq!(h2.height, 2);
	assert_eq!(h2.parent, hash(&h1));
	assert_eq!(h2.extrinsics_root, hash(&[10, 20]));
	assert_eq!(h2.state, 36);
}

#[test]
fn bc_4_verify_three_blocks() {
	let g = Block::genesis();
	let b1 = g.child(vec![1]);
	let b2 = b1.child(vec![2]);
	let chain = vec![g.clone(), b1, b2];
	assert!(g.verify_sub_chain(&chain[1..]));
}

#[test]
fn bc_4_invalid_header_does_not_check() {
	let g = Header::genesis();
	let h1 = Header { parent: 0, height: 100, extrinsics_root: 0, state: 100, consensus_digest: 0 };

	assert!(!g.verify_child(&h1));
}

#[test]
fn bc_4_invalid_block_state_does_not_check() {
	let b0 = Block::genesis();
	let mut b1 = b0.child(vec![1, 2, 3]);
	b1.body = vec![];

	assert!(!b0.verify_sub_chain(&[b1]));
}

#[test]
fn bc_4_block_with_invalid_header_does_not_check() {
	let b0 = Block::genesis();
	let mut b1 = b0.child(vec![1, 2, 3]);
	b1.header = Header::genesis();

	assert!(!b0.verify_sub_chain(&[b1]));
}

#[test]
fn bc_4_student_invalid_block_really_is_invalid() {
	let gb = Block::genesis();
	let gh = &gb.header;

	let b1 = build_invalid_child_block_with_valid_header(gh);
	let h1 = &b1.header;

	// Make sure that the header is valid according to header rules.
	assert!(gh.verify_child(h1));

	// Make sure that the block is not valid when executed.
	assert!(!gb.verify_sub_chain(&[b1]));
}
