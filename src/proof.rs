//! The proof object model for RPN proofs used in Metamath.

use diag::Diagnostic;
use nameck::Nameset;
use parser::StatementAddress;
use parser::StatementRef;
use scopeck::ScopeResult;
use segment_set::SegmentSet;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::hash::SipHasher;
use std::sync::Arc;
use verify::ProofBuilder;
use verify::verify_one;

/// A tree structure for storing proofs and grammar derivations.
#[derive(Clone,Debug,Eq)]
pub struct ProofTree {
    /// The axiom/theorem being applied at the root.
    pub address: StatementAddress,
    /// The hypotheses ($e and $f) in database order.
    pub children: Vec<Arc<ProofTree>>,
    /// The precomputed hash for this tree.
    hash: u64,
}

impl PartialEq for ProofTree {
    fn eq(&self, other: &ProofTree) -> bool {
        self.address == other.address && self.children == other.children
    }
}

impl Hash for ProofTree {
    fn hash<H>(&self, state: &mut H)
        where H: Hasher
    {
        self.hash.hash(state)
    }
}

impl ProofTree {
    /// Create a new proof tree using the given atom and children.
    pub fn new(address: StatementAddress, children: Vec<Arc<ProofTree>>) -> Self {
        let mut hasher = SipHasher::new();
        address.hash(&mut hasher);
        children.hash(&mut hasher);
        ProofTree {
            address: address,
            children: children,
            hash: hasher.finish(),
        }
    }
}


/// An array of proof trees, used to collect steps of a proof
/// in proof order
#[derive(Default,Debug,Clone)]
pub struct ProofTreeArray {
    map: HashMap<u64, usize>,
    /// The list of proof trees
    pub trees: Vec<Arc<ProofTree>>,
    /// The uncompressed strings for each proof tree
    pub exprs: Vec<Vec<u8>>,
}

impl ProofTreeArray {
    /// Get the index of a proof tree in the array
    pub fn index(&self, tree: &ProofTree) -> Option<usize> {
        self.map.get(&tree.hash).cloned()
    }

    /// Create a proof tree array from the proof  a single $p statement, returning the result of the given
    /// proof builder, or an error if the proof is faulty
    pub fn new(sset: &SegmentSet,
               nset: &Nameset,
               scopes: &ScopeResult,
               stmt: StatementRef)
               -> Result<(ProofTreeArray, usize), Diagnostic> {
        let mut arr = ProofTreeArray::default();
        let arc = try!(verify_one(sset, nset, scopes, &mut arr, stmt));
        let qed = arr.index(&arc).unwrap();
        Ok((arr, qed))
    }
}

impl ProofBuilder for ProofTreeArray {
    type Item = Arc<ProofTree>;

    fn build(&mut self, addr: StatementAddress, trees: Vec<Arc<ProofTree>>, expr: &[u8]) -> Arc<ProofTree> {
        let tree = ProofTree::new(addr, trees);
        match self.index(&tree) {
            Some(n) => self.trees[n].clone(),
            None => {
                self.map.insert(tree.hash, self.trees.len());
                let arc = Arc::new(tree);
                self.trees.push(arc.clone());
                let mut uexpr = vec![b' '];
                for &chr in expr {
                    if chr & 0x80 == 0 {
                        uexpr.push(chr);
                    } else {
                        uexpr.push(chr & 0x7F);
                        uexpr.push(b' ');
                    }
                }
                uexpr.pop();
                self.exprs.push(uexpr);
                arc
            }
        }
    }
}
