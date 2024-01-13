//! PoW and PoA each have their own set of strengths and weaknesses. Many chains are happy to choose
//! one of them. But other chains would like consensus properties that fall in between. To achieve
//! this we could consider interleaving PoW blocks with PoA blocks. Some very early designs of
//! Ethereum considered this approach as a way to transition away from PoW.

use super::Consensus;
use super::Hash;
use super::Header;

//For selecting which between two in a clear way
enum Which {
    First,
    Second
}

//A header structure that contains two digests
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DoubleHeader<Digest1, Digest2> {
	parent: Hash,
	height: u64,
	state_root: Hash,
	extrinsics_root: Hash,
	consensus_digest: Digest1,
	consensus_digest2: Digest2,
}

//convert a double header to an consensus engine consumable one
impl<Digest1, Digest2> DoubleHeader<Digest1, Digest2> {
	fn to_header<T>(&self, digest: T) -> Header<T> {
		Header {
			parent: self.parent,
			height: self.height,
			state_root: self.state_root,
			extrinsics_root: self.extrinsics_root,
			consensus_digest: digest,
		}
	}
}

/// A Double consensus engine that alternates back and forth between PoW and PoA sealed blocks
/// Really only works for no pre digest necessary consensus engines
/// Could also make generic but I'll leave it for now
struct DoubleEngine<E1: Consensus, E2: Consensus>(E1, E2);

impl<Engine1: Consensus, Engine2: Consensus> DoubleEngine<Engine1, Engine2> {
	
    fn validate(&self, header: &DoubleHeader<Engine1::Digest, Engine2::Digest>) -> bool {
        let which = Self::validate_with_which(&header);
        let passed = match which {
            Which::First => self.0.validate(&Engine1::Digest::default(), 
                    &header.to_header(header.consensus_digest)),
            Which::Second => self.1.validate(&Engine2::Digest::default(), 
                    &header.to_header(header.consensus_digest2)), 
        };
        passed
	}

    //define rules for which engine validates
    fn validate_with_which<Digest1, Digest2>(header: &DoubleHeader<Digest1, Digest2>) -> Which {
        match header.height % 2 == 0 {
            true => Which::First,
            false => Which::Second
        }
    }

	fn seal(
		    &self,
		    partial_header: DoubleHeader<(), ()>,
	    ) -> Option<DoubleHeader<Engine1::Digest, Engine2::Digest>> {
	    
        //can use opposite of validate_with_which since alternating
        let which = Self::validate_with_which(&partial_header);
        let header = match which {
            Which::First => self.1.seal(&Engine2::Digest::default(), 
            &partial_header.to_header(partial_header.consensus_digest2)), 
            // Which::Second => self.0.seal(&Engine1::Digest::default(), 
            // &partial_header.to_header(header.consensus_digest)),, 
        };

        match header {
            Some(h) => h,
            None => None
        }
	}
}
