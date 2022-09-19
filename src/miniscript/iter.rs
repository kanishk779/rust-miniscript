// Miniscript
// Written in 2020 by
//     Dr Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! Miniscript Iterators
//!
//! Iterators for Miniscript with special functions for iterating
//! over Public Keys, Public Key Hashes or both.
use core::ops::Deref;

use sync::Arc;

use super::decode::Terminal;
use super::{Miniscript, MiniscriptKey, ScriptContext};
use crate::miniscript::musig_key::KeyExprIter;
use crate::prelude::*;

/// Iterator-related extensions for [Miniscript]
impl<Pk: MiniscriptKey, Ctx: ScriptContext> Miniscript<Pk, Ctx> {
    /// Creates a new [Iter] iterator that will iterate over all [Miniscript] items within
    /// AST by traversing its branches. For the specific algorithm please see
    /// [Iter::next] function.
    pub fn iter(&self) -> Iter<Pk, Ctx> {
        Iter::new(self)
    }

    /// Creates a new [PkIter] iterator that will iterate over all plain public keys (and not
    /// key hash values) present in [Miniscript] items within AST by traversing all its branches.
    /// For the specific algorithm please see [PkIter::next] function.
    pub fn iter_pk(&self) -> PkIter<Pk, Ctx> {
        PkIter::new(self)
    }

    /// Creates a new [PkhIter] iterator that will iterate over all public keys hashes (and not
    /// plain public keys) present in Miniscript items within AST by traversing all its branches.
    /// For the specific algorithm please see [PkhIter::next] function.
    pub fn iter_pkh(&self) -> PkhIter<Pk, Ctx> {
        PkhIter::new(self)
    }

    /// Creates a new [PkPkhIter] iterator that will iterate over all plain public keys and
    /// key hash values present in Miniscript items within AST by traversing all its branches.
    /// For the specific algorithm please see [PkPkhIter::next] function.
    pub fn iter_pk_pkh(&self) -> PkPkhIter<Pk, Ctx> {
        PkPkhIter::new(self)
    }

    /// Enumerates all child nodes of the current AST node (`self`) and returns a `Vec` referencing
    /// them.
    pub fn branches(&self) -> Vec<&Miniscript<Pk, Ctx>> {
        match self.node {
            Terminal::PkK(_) | Terminal::PkH(_) | Terminal::RawPkH(_) | Terminal::Multi(_, _) => {
                vec![]
            }

            Terminal::Alt(ref node)
            | Terminal::Swap(ref node)
            | Terminal::Check(ref node)
            | Terminal::DupIf(ref node)
            | Terminal::Verify(ref node)
            | Terminal::NonZero(ref node)
            | Terminal::ZeroNotEqual(ref node) => vec![node],

            Terminal::AndV(ref node1, ref node2)
            | Terminal::AndB(ref node1, ref node2)
            | Terminal::OrB(ref node1, ref node2)
            | Terminal::OrD(ref node1, ref node2)
            | Terminal::OrC(ref node1, ref node2)
            | Terminal::OrI(ref node1, ref node2) => vec![node1, node2],

            Terminal::AndOr(ref node1, ref node2, ref node3) => vec![node1, node2, node3],

            Terminal::Thresh(_, ref node_vec) => node_vec.iter().map(Arc::deref).collect(),

            _ => vec![],
        }
    }

    /// Returns child node with given index, if any
    pub fn get_nth_child(&self, n: usize) -> Option<&Miniscript<Pk, Ctx>> {
        match (n, &self.node) {
            (0, &Terminal::Alt(ref node))
            | (0, &Terminal::Swap(ref node))
            | (0, &Terminal::Check(ref node))
            | (0, &Terminal::DupIf(ref node))
            | (0, &Terminal::Verify(ref node))
            | (0, &Terminal::NonZero(ref node))
            | (0, &Terminal::ZeroNotEqual(ref node))
            | (0, &Terminal::AndV(ref node, _))
            | (0, &Terminal::AndB(ref node, _))
            | (0, &Terminal::OrB(ref node, _))
            | (0, &Terminal::OrD(ref node, _))
            | (0, &Terminal::OrC(ref node, _))
            | (0, &Terminal::OrI(ref node, _))
            | (1, &Terminal::AndV(_, ref node))
            | (1, &Terminal::AndB(_, ref node))
            | (1, &Terminal::OrB(_, ref node))
            | (1, &Terminal::OrD(_, ref node))
            | (1, &Terminal::OrC(_, ref node))
            | (1, &Terminal::OrI(_, ref node))
            | (0, &Terminal::AndOr(ref node, _, _))
            | (1, &Terminal::AndOr(_, ref node, _))
            | (2, &Terminal::AndOr(_, _, ref node)) => Some(node),

            (n, &Terminal::Thresh(_, ref node_vec)) => node_vec.get(n).map(|x| &**x),

            _ => None,
        }
    }

    /// Returns `Vec` with cloned version of all public keys from the current miniscript item,
    /// if any. Otherwise returns an empty `Vec`.
    ///
    /// NB: The function analyzes only single miniscript item and not any of its descendants in AST.
    /// To obtain a list of all public keys within AST use [Miniscript::iter_pk()] function, for example
    /// `miniscript.iter_pubkeys().collect()`.
    pub fn get_leapk(&self) -> Vec<Pk> {
        match self.node {
            Terminal::PkK(ref key) => key.iter().map(|pk| pk.clone()).collect(),
            Terminal::PkH(ref key) => vec![key.clone()],
            Terminal::Multi(_, ref keys) => keys.clone(),
            Terminal::MultiA(_, ref keys) => {
                let mut res: Vec<Pk> = Vec::<Pk>::new();
                for key in keys {
                    res.extend(key.iter().cloned());
                }
                res
            }
            _ => vec![],
        }
    }

    /// Returns `Vec` with hashes of all public keys from the current miniscript item, if any.
    /// Otherwise returns an empty `Vec`.
    ///
    /// For each public key the function computes hash; for each hash of the public key the function
    /// returns its cloned copy.
    ///
    /// NB: The function analyzes only single miniscript item and not any of its descendants in AST.
    /// To obtain a list of all public key hashes within AST use [Miniscript::iter_pkh()] function,
    /// for example `miniscript.iter_pubkey_hashes().collect()`.
    pub fn get_leapkh(&self) -> Vec<Pk::RawPkHash> {
        match self.node {
            Terminal::RawPkH(ref hash) => vec![hash.clone()],
            Terminal::PkH(ref key) => vec![key.to_pubkeyhash()],
            Terminal::PkK(ref key) => key.iter().map(|pk| pk.to_pubkeyhash()).collect(),
            Terminal::Multi(_, ref keys) => keys.iter().map(Pk::to_pubkeyhash).collect(),
            Terminal::MultiA(_, ref keys) => {
                let mut res: Vec<Pk::RawPkHash> = Vec::<Pk::RawPkHash>::new();
                for key in keys {
                    res.extend(key.iter().map(|pk| pk.to_pubkeyhash()));
                }
                res
            }
            _ => vec![],
        }
    }

    /// Returns `Vec` of [PkPkh] entries, representing either public keys or public key
    /// hashes, depending on the data from the current miniscript item. If there is no public
    /// keys or hashes, the function returns an empty `Vec`.
    ///
    /// NB: The function analyzes only single miniscript item and not any of its descendants in AST.
    /// To obtain a list of all public keys or hashes within AST use [Miniscript::iter_pk_pkh()]
    /// function, for example `miniscript.iter_pubkeys_and_hashes().collect()`.
    pub fn get_leapk_pkh(&self) -> Vec<PkPkh<Pk>> {
        match self.node {
            Terminal::RawPkH(ref hash) => vec![PkPkh::HashedPubkey(hash.clone())],
            Terminal::PkH(ref key) => vec![PkPkh::PlainPubkey(key.clone())],
            Terminal::PkK(ref key) => key
                .iter()
                .map(|pk| PkPkh::PlainPubkey(pk.clone()))
                .collect(),
            Terminal::Multi(_, ref keys) => keys
                .iter()
                .map(|key| PkPkh::PlainPubkey(key.clone()))
                .collect(),
            Terminal::MultiA(_, ref keys) => {
                let mut res: Vec<PkPkh<Pk>> = Vec::<PkPkh<Pk>>::new();
                for key in keys {
                    res.extend(key.iter().map(|pk| PkPkh::PlainPubkey(pk.clone())));
                }
                res
            }
            _ => vec![],
        }
    }

    /// Returns `Option::Some` with cloned n'th public key from the current miniscript item,
    /// if any. Otherwise returns `Option::None`.
    ///
    /// NB: The function analyzes only single miniscript item and not any of its descendants in AST.

    pub fn get_nth_pk(&self, n: usize) -> Option<Pk> {
        match (&self.node, n) {
            (&Terminal::PkH(ref key), 0) => Some(key.clone()),
            (&Terminal::Multi(_, ref keys), _) => {
                if n < keys.len() {
                    Some(keys[n].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    /// Returns `Option::Some` with hash of n'th public key from the current miniscript item,
    /// if any. Otherwise returns `Option::None`.
    ///
    /// For each public key the function computes hash; for each hash of the public key the function
    /// returns it cloned copy.
    ///
    /// NB: The function analyzes only single miniscript item and not any of its descendants in AST.

    pub fn get_nth_pkh(&self, n: usize) -> Option<Pk::RawPkHash> {
        match (&self.node, n) {
            (&Terminal::RawPkH(ref hash), 0) => Some(hash.clone()),
            (&Terminal::PkH(ref key), 0) => Some(key.to_pubkeyhash()),
            (&Terminal::Multi(_, ref keys), _) => {
                if n < keys.len() {
                    Some(keys[n].to_pubkeyhash())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Returns `Option::Some` with hash of n'th public key or hash from the current miniscript item,
    /// if any. Otherwise returns `Option::None`.
    ///
    /// NB: The function analyzes only single miniscript item and not any of its descendants in AST.

    pub fn get_nth_pk_pkh(&self, n: usize) -> Option<PkPkh<Pk>> {
        match (&self.node, n) {
            (&Terminal::RawPkH(ref hash), 0) => Some(PkPkh::HashedPubkey(hash.clone())),
            (&Terminal::PkH(ref key), 0) => Some(PkPkh::PlainPubkey(key.clone())),
            (&Terminal::Multi(_, ref keys), _) => {
                if n < keys.len() {
                    Some(PkPkh::PlainPubkey(keys[n].clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
/// Parent iter for all the below iters
struct BaseIter<'a, Pk: MiniscriptKey, Ctx: ScriptContext> {
    node_iter: Iter<'a, Pk, Ctx>,
    curr_node: Option<&'a Miniscript<Pk, Ctx>>,
    // If current fragment is PkK or MultiA, then this is the iterator over public keys
    musig_iter: Option<KeyExprIter<'a, Pk>>,
    // Helps in checking whether current fragment is MultiA or not,
    // additionally provides the length of vec<keyexpr>
    multi_a_len: Option<u32>,
    key_index: usize,
}

impl<'a, Pk: MiniscriptKey, Ctx: ScriptContext> BaseIter<'a, Pk, Ctx> {
    fn goto_next_node(&mut self) -> () {
        self.curr_node = self.node_iter.next();
        self.key_index = 0;
        let mut multi_a_len = None;
        self.musig_iter = match self.curr_node {
            Some(script) => match script.node {
                Terminal::PkK(ref pk) => {
                    multi_a_len = Some(1 as u32);
                    Some(pk.iter())
                }
                Terminal::MultiA(_, ref keys) => {
                    multi_a_len = Some(keys.len() as u32);
                    Some(keys[0].iter())
                }
                _ => None,
            },
            None => None,
        };
        self.multi_a_len = multi_a_len;
    }
}
/// Iterator for traversing all [Miniscript] miniscript AST references starting from some specific
/// node which constructs the iterator via [Miniscript::iter] method.
pub struct Iter<'a, Pk: MiniscriptKey, Ctx: ScriptContext> {
    next: Option<&'a Miniscript<Pk, Ctx>>,
    // Here we store vec of path elements, where each element is a tuple, consisting of:
    // 1. Miniscript node on the path
    // 2. Index of the current branch
    path: Vec<(&'a Miniscript<Pk, Ctx>, usize)>,
}

impl<'a, Pk: MiniscriptKey, Ctx: ScriptContext> Iter<'a, Pk, Ctx> {
    fn new(miniscript: &'a Miniscript<Pk, Ctx>) -> Self {
        Iter {
            next: Some(miniscript),
            path: vec![],
        }
    }
}

impl<'a, Pk: MiniscriptKey, Ctx: ScriptContext> Iterator for Iter<'a, Pk, Ctx> {
    type Item = &'a Miniscript<Pk, Ctx>;

    /// First, the function returns `self`, then the first child of the self (if any),
    /// then proceeds to the child of the child — down to a leaf of the tree in its first branch.
    /// When the leaf is reached, it goes in the reverse direction on the same branch until it
    /// founds a first branching node that had more than a single branch and returns it, traversing
    /// it with the same algorithm again.
    ///
    /// For example, for the given AST
    /// ```text
    /// A --+--> B -----> C --+--> D -----> E
    ///     |                 |
    ///     |                 +--> F
    ///     |                 |
    ///     |                 +--> G --+--> H
    ///     |                          |
    ///     |                          +--> I -----> J
    ///     +--> K
    /// ```
    /// `Iter::next()` will iterate over the nodes in the following order:
    /// `A > B > C > D > E > F > G > H > I > J > K`
    ///
    /// To enumerate the branches iterator uses [Miniscript::branches] function.
    fn next(&mut self) -> Option<Self::Item> {
        let mut curr = self.next;
        if curr.is_none() {
            while let Some((node, child)) = self.path.pop() {
                curr = node.get_nth_child(child);
                if curr.is_some() {
                    self.path.push((node, child + 1));
                    break;
                }
            }
        }
        if let Some(node) = curr {
            self.next = node.get_nth_child(0);
            self.path.push((node, 1));
        }
        curr
    }
}
/// Iterator for traversing all [MiniscriptKey]'s in AST starting from some specific node which
/// constructs the iterator via [Miniscript::iter_pk] method.
pub struct PkIter<'a, Pk: MiniscriptKey, Ctx: ScriptContext> {
    base_iter: BaseIter<'a, Pk, Ctx>,
}

impl<'a, Pk: MiniscriptKey, Ctx: ScriptContext> PkIter<'a, Pk, Ctx> {
    fn new(miniscript: &'a Miniscript<Pk, Ctx>) -> Self {
        let mut iter = Iter::new(miniscript);
        let curr_node = iter.next();
        let mut multi_a_len = None;
        let musig_iter = match curr_node {
            Some(script) => match script.node {
                Terminal::PkK(ref pk) => {
                    multi_a_len = Some(1 as u32);
                    Some(pk.iter())
                }
                Terminal::MultiA(_, ref keys) => {
                    multi_a_len = Some(keys.len() as u32);
                    Some(keys[0].iter())
                }
                _ => None,
            },
            None => None,
        };
        let bs_iter = BaseIter {
            curr_node: curr_node,
            node_iter: iter,
            musig_iter: musig_iter,
            key_index: 0,
            multi_a_len: multi_a_len,
        };
        PkIter { base_iter: bs_iter }
    }
}

impl<'a, Pk: MiniscriptKey, Ctx: ScriptContext> Iterator for PkIter<'a, Pk, Ctx> {
    type Item = Pk;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.base_iter.curr_node {
                None => break None,
                Some(script) => match &script.node {
                    Terminal::PkK(_) => {
                        // check if musig_iter has something
                        match self.base_iter.musig_iter.as_mut().unwrap().next() {
                            Some(pk) => break Some(pk.clone()),
                            None => {
                                self.base_iter.goto_next_node();
                                continue;
                            }
                        }
                    }
                    Terminal::MultiA(_, keys) => {
                        match self.base_iter.musig_iter.as_mut().unwrap().next() {
                            Some(pk) => break Some(pk.clone()),
                            None => {
                                // When the current iterator has yielded all the keys
                                let vec_size = self.base_iter.multi_a_len.unwrap();
                                self.base_iter.key_index += 1;
                                if (self.base_iter.key_index as u32) < vec_size {
                                    // goto the next KeyExpr in the vector
                                    self.base_iter.musig_iter =
                                        Some(keys[self.base_iter.key_index].iter());
                                    match self.base_iter.musig_iter.as_mut().unwrap().next() {
                                        None => {
                                            self.base_iter.key_index += 1;
                                            continue;
                                        }
                                        Some(pk) => break Some(pk.clone()),
                                    }
                                } else {
                                    // if we have exhausted all the KeyExpr
                                    self.base_iter.goto_next_node();
                                    continue;
                                }
                            }
                        }
                    }
                    _ => match script.get_nth_pk(self.base_iter.key_index) {
                        Some(pk) => {
                            self.base_iter.key_index += 1;
                            break Some(pk);
                        }
                        None => {
                            self.base_iter.goto_next_node();
                            continue;
                        }
                    },
                },
            }
        }
    }
}

/// Iterator for traversing all [MiniscriptKey] hashes in AST starting from some specific node which
/// constructs the iterator via [Miniscript::iter_pkh] method.
pub struct PkhIter<'a, Pk: MiniscriptKey, Ctx: ScriptContext> {
    base_iter: BaseIter<'a, Pk, Ctx>,
}

impl<'a, Pk: MiniscriptKey, Ctx: ScriptContext> PkhIter<'a, Pk, Ctx> {
    fn new(miniscript: &'a Miniscript<Pk, Ctx>) -> Self {
        let mut iter = Iter::new(miniscript);
        let curr_node = iter.next();
        let mut multi_a_len = None;
        let musig_iter = match curr_node {
            Some(script) => match script.node {
                Terminal::PkK(ref pk) => {
                    multi_a_len = Some(1 as u32);
                    Some(pk.iter())
                }
                Terminal::MultiA(_, ref keys) => {
                    multi_a_len = Some(keys.len() as u32);
                    Some(keys[0].iter())
                }
                _ => None,
            },
            None => None,
        };
        let bs_iter = BaseIter {
            curr_node: curr_node,
            node_iter: iter,
            musig_iter: musig_iter,
            multi_a_len: multi_a_len,
            key_index: 0,
        };
        PkhIter { base_iter: bs_iter }
    }
}

impl<'a, Pk: MiniscriptKey, Ctx: ScriptContext> Iterator for PkhIter<'a, Pk, Ctx> {
    type Item = Pk::RawPkHash;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.base_iter.curr_node {
                None => break None,
                Some(script) => match &script.node {
                    Terminal::PkK(_) => {
                        // check if musig_iter has something
                        match self.base_iter.musig_iter.as_mut().unwrap().next() {
                            Some(pk) => break Some(pk.to_pubkeyhash()),
                            None => {
                                self.base_iter.goto_next_node();
                                continue;
                            }
                        }
                    }
                    Terminal::MultiA(_, keys) => {
                        match self.base_iter.musig_iter.as_mut().unwrap().next() {
                            Some(pk) => break Some(pk.to_pubkeyhash()),
                            None => {
                                // When the current iterator has yielded all the keys
                                let vec_size = self.base_iter.multi_a_len.unwrap();
                                self.base_iter.key_index += 1;
                                if (self.base_iter.key_index as u32) < vec_size {
                                    // goto the next KeyExpr in the vector
                                    self.base_iter.musig_iter =
                                        Some(keys[self.base_iter.key_index].iter());
                                    match self.base_iter.musig_iter.as_mut().unwrap().next() {
                                        None => {
                                            self.base_iter.key_index += 1;
                                            continue;
                                        }
                                        Some(pk) => break Some(pk.to_pubkeyhash()),
                                    }
                                } else {
                                    // if we have exhausted all the KeyExpr
                                    self.base_iter.goto_next_node();
                                    continue;
                                }
                            }
                        }
                    }
                    _ => match script.get_nth_pkh(self.base_iter.key_index) {
                        Some(pk) => {
                            self.base_iter.key_index += 1;
                            break Some(pk);
                        }
                        None => {
                            self.base_iter.goto_next_node();
                            continue;
                        }
                    },
                },
            }
        }
    }
}

/// Enum representing either key or a key hash value coming from a miniscript item inside AST
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PkPkh<Pk: MiniscriptKey> {
    /// Plain public key
    PlainPubkey(Pk),
    /// Hashed public key
    HashedPubkey(Pk::RawPkHash),
}

impl<Pk: MiniscriptKey<RawPkHash = Pk>> PkPkh<Pk> {
    /// Convenience method to avoid distinguishing between keys and hashes when these are the same type
    pub fn as_key(self) -> Pk {
        match self {
            PkPkh::PlainPubkey(pk) => pk,
            PkPkh::HashedPubkey(pkh) => pkh,
        }
    }
}

/// Iterator for traversing all [MiniscriptKey]'s and hashes, depending what data are present in AST,
/// starting from some specific node which constructs the iterator via
/// [Miniscript::iter_pk_pkh] method.
pub struct PkPkhIter<'a, Pk: MiniscriptKey, Ctx: ScriptContext> {
    base_iter: BaseIter<'a, Pk, Ctx>,
}

impl<'a, Pk: MiniscriptKey, Ctx: ScriptContext> PkPkhIter<'a, Pk, Ctx> {
    fn new(miniscript: &'a Miniscript<Pk, Ctx>) -> Self {
        let mut iter = Iter::new(miniscript);
        let curr_node = iter.next();
        let mut multi_a_len = None;
        let musig_iter = match curr_node {
            Some(script) => match script.node {
                Terminal::PkK(ref pk) => {
                    multi_a_len = Some(1 as u32);
                    Some(pk.iter())
                }
                Terminal::MultiA(_, ref keys) => {
                    multi_a_len = Some(keys.len() as u32);
                    Some(keys[0].iter())
                }
                _ => None,
            },
            None => None,
        };
        let bs_iter = BaseIter {
            curr_node: curr_node,
            node_iter: iter,
            key_index: 0,
            musig_iter: musig_iter,
            multi_a_len: multi_a_len,
        };
        PkPkhIter { base_iter: bs_iter }
    }

    /// Returns a `Option`, listing all public keys found in AST starting from this
    /// `Miniscript` item, or `None` signifying that at least one key hash was found, making
    /// impossible to enumerate all source public keys from the script.
    ///
    /// * Differs from `Miniscript::iter_pubkeys().collect()` in the way that this function fails on
    ///   the first met public key hash, while [PkIter] just ignores them.
    /// * Differs from `Miniscript::iter_pubkeys_and_hashes().collect()` in the way that it lists
    ///   only public keys, and not their hashes
    ///
    /// Unlike these functions, [PkPkhIter::pk_only] returns an `Option` value with `Vec`, not an iterator,
    /// and consumes the iterator object.
    pub fn pk_only(self) -> Option<Vec<Pk>> {
        let mut keys = vec![];
        for item in self {
            match item {
                PkPkh::HashedPubkey(_) => return None,
                PkPkh::PlainPubkey(key) => {
                    keys.push(key);
                }
            }
        }
        Some(keys)
    }
}

impl<'a, Pk: MiniscriptKey, Ctx: ScriptContext> Iterator for PkPkhIter<'a, Pk, Ctx> {
    type Item = PkPkh<Pk>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.base_iter.curr_node {
                None => break None,
                Some(script) => match &script.node {
                    Terminal::PkK(_) => {
                        // check if musig_iter has something
                        match self.base_iter.musig_iter.as_mut().unwrap().next() {
                            Some(pk) => break Some(PkPkh::PlainPubkey(pk.clone())),
                            None => {
                                self.base_iter.goto_next_node();
                                continue;
                            }
                        }
                    }
                    Terminal::MultiA(_, keys) => {
                        match self.base_iter.musig_iter.as_mut().unwrap().next() {
                            Some(pk) => break Some(PkPkh::PlainPubkey(pk.clone())),
                            None => {
                                // When the current iterator has yielded all the keys
                                let vec_size = self.base_iter.multi_a_len.unwrap();
                                self.base_iter.key_index += 1;
                                if (self.base_iter.key_index as u32) < vec_size {
                                    // goto the next KeyExpr in the vector
                                    self.base_iter.musig_iter =
                                        Some(keys[self.base_iter.key_index].iter());
                                    match self.base_iter.musig_iter.as_mut().unwrap().next() {
                                        None => {
                                            self.base_iter.key_index += 1;
                                            continue;
                                        }
                                        Some(pk) => break Some(PkPkh::PlainPubkey(pk.clone())),
                                    }
                                } else {
                                    // if we have exhausted all the KeyExpr
                                    self.base_iter.goto_next_node();
                                    continue;
                                }
                            }
                        }
                    }
                    _ => match script.get_nth_pk_pkh(self.base_iter.key_index) {
                        Some(pk) => {
                            self.base_iter.key_index += 1;
                            break Some(pk.clone());
                        }
                        None => {
                            self.base_iter.goto_next_node();
                            continue;
                        }
                    },
                },
            }
        }
    }
}

// Module is public since it export testcase generation which may be used in
// dependent libraries for their own tasts based on Miniscript AST
#[cfg(test)]
pub mod test {
    use core::str::FromStr;

    use bitcoin;
    use bitcoin::hashes::{hash160, ripemd160, sha256, sha256d, Hash};
    use bitcoin::secp256k1;

    use super::{Miniscript, PkIter, PkPkh};
    use crate::miniscript::context::{Segwitv0, Tap};
    type Segwitv0String = Miniscript<String, Segwitv0>;
    type TapscriptString = Miniscript<String, Tap>;

    pub type TestData = (
        Miniscript<bitcoin::PublicKey, Segwitv0>,
        Vec<bitcoin::PublicKey>,
        Vec<hash160::Hash>,
        bool, // Indicates that the top-level contains public key or hashes
    );

    pub fn gen_secp_pubkeys(n: usize) -> Vec<secp256k1::PublicKey> {
        let mut ret = Vec::with_capacity(n);
        let secp = secp256k1::Secp256k1::new();
        let mut sk = [0; 32];

        for i in 1..n + 1 {
            sk[0] = i as u8;
            sk[1] = (i >> 8) as u8;
            sk[2] = (i >> 16) as u8;

            ret.push(secp256k1::PublicKey::from_secret_key(
                &secp,
                &secp256k1::SecretKey::from_slice(&sk[..]).unwrap(),
            ));
        }
        ret
    }

    pub fn gen_bitcoin_pubkeys(n: usize, compressed: bool) -> Vec<bitcoin::PublicKey> {
        gen_secp_pubkeys(n)
            .into_iter()
            .map(|inner| bitcoin::PublicKey { inner, compressed })
            .collect()
    }

    pub fn gen_testcases() -> Vec<TestData> {
        let k = gen_bitcoin_pubkeys(10, true);
        let _h: Vec<hash160::Hash> = k
            .iter()
            .map(|pk| hash160::Hash::hash(&pk.to_bytes()))
            .collect();

        let preimage = vec![0xab as u8; 32];
        let sha256_hash = sha256::Hash::hash(&preimage);
        let sha256d_hash_rev = sha256d::Hash::hash(&preimage);
        let mut sha256d_hash_bytes = sha256d_hash_rev.clone().into_inner();
        sha256d_hash_bytes.reverse();
        let sha256d_hash = sha256d::Hash::from_inner(sha256d_hash_bytes);
        let hash160_hash = hash160::Hash::hash(&preimage);
        let ripemd160_hash = ripemd160::Hash::hash(&preimage);

        vec![
            (ms_str!("after({})", 1000), vec![], vec![], false),
            (ms_str!("older({})", 1000), vec![], vec![], false),
            (ms_str!("sha256({})", sha256_hash), vec![], vec![], false),
            (ms_str!("hash256({})", sha256d_hash), vec![], vec![], false),
            (ms_str!("hash160({})", hash160_hash), vec![], vec![], false),
            (
                ms_str!("ripemd160({})", ripemd160_hash),
                vec![],
                vec![],
                false,
            ),
            (ms_str!("c:pk_k({})", k[0]), vec![k[0]], vec![], true),
            (ms_str!("c:pk_h({})", k[0]), vec![k[0]], vec![], true),
            (
                ms_str!("and_v(vc:pk_k({}),c:pk_h({}))", k[0], k[1]),
                vec![k[0], k[1]],
                vec![],
                false,
            ),
            (
                ms_str!("and_b(c:pk_k({}),sjtv:sha256({}))", k[0], sha256_hash),
                vec![k[0]],
                vec![],
                false,
            ),
            (
                ms_str!(
                    "andor(c:pk_k({}),jtv:sha256({}),c:pk_h({}))",
                    k[1],
                    sha256_hash,
                    k[2]
                ),
                vec![k[1], k[2]],
                vec![],
                false,
            ),
            (
                ms_str!("multi(3,{},{},{},{},{})", k[9], k[8], k[7], k[0], k[1]),
                vec![k[9], k[8], k[7], k[0], k[1]],
                vec![],
                true,
            ),
            (
                ms_str!(
                    "thresh(3,c:pk_k({}),sc:pk_k({}),sc:pk_k({}),sc:pk_k({}),sc:pk_k({}))",
                    k[2],
                    k[3],
                    k[4],
                    k[5],
                    k[6]
                ),
                vec![k[2], k[3], k[4], k[5], k[6]],
                vec![],
                false,
            ),
            (
                ms_str!(
                    "or_d(multi(2,{},{}),and_v(v:multi(2,{},{}),older(10000)))",
                    k[6],
                    k[7],
                    k[8],
                    k[9]
                ),
                vec![k[6], k[7], k[8], k[9]],
                vec![],
                false,
            ),
            (
                ms_str!(
                    "or_d(multi(3,{},{},{},{},{}),\
                      and_v(v:thresh(2,c:pk_h({}),\
                      ac:pk_h({}),ac:pk_h({})),older(10000)))",
                    k[0],
                    k[2],
                    k[4],
                    k[6],
                    k[9],
                    k[1],
                    k[3],
                    k[5]
                ),
                vec![k[0], k[2], k[4], k[6], k[9], k[1], k[3], k[5]],
                vec![],
                false,
            ),
        ]
    }

    #[test]
    fn test_musig_iter() {
        // TEST: musig inside Pk in Tap context
        let ms = TapscriptString::from_str("or_b(pk(A),s:pk(musig(B,C)))").unwrap();
        let mut pk_iter = PkIter::new(&ms);
        let keys = vec!["A", "B", "C"];
        for key in keys {
            assert_eq!(String::from(key), pk_iter.next().unwrap());
        }

        // TEST: complex musig in Tap context
        let ms =
            TapscriptString::from_str("or_b(pk(A),s:pk(musig(F,B,musig(C,musig(D,E)))))").unwrap();
        let mut pk_iter = PkIter::new(&ms);
        let keys = vec!["A", "F", "B", "C", "D", "E"];
        for key in keys {
            assert_eq!(String::from(key), pk_iter.next().unwrap());
        }

        // TEST: without musig in segwit context
        let ms = Segwitv0String::from_str("or_b(pk(A),s:pk(B))").unwrap();
        let mut pk_iter = PkIter::new(&ms);
        let keys = vec!["A", "B"];
        for key in keys {
            assert_eq!(String::from(key), pk_iter.next().unwrap());
        }

        // TEST: musig inside multi_a in Tap context
        let ms = TapscriptString::from_str("or_b(pk(A),a:multi_a(1,B,musig(C,D)))").unwrap();
        let mut pk_iter = PkIter::new(&ms);
        let keys = vec!["A", "B", "C", "D"];
        for key in keys {
            assert_eq!(String::from(key), pk_iter.next().unwrap());
        }

        // TEST: musig and normal key in Tap context
        let ms =
            TapscriptString::from_str("or_b(pk(musig(A1,A2)),a:multi_a(1,B,musig(C,musig(D,E))))")
                .unwrap();
        let mut pk_iter = PkIter::new(&ms);
        let keys = vec!["A1", "A2", "B", "C", "D", "E"];
        for key in keys {
            assert_eq!(String::from(key), pk_iter.next().unwrap());
        }
    }
    #[test]
    fn get_keys() {
        gen_testcases()
            .into_iter()
            .for_each(|(ms, k, _, test_top_level)| {
                if !test_top_level {
                    return;
                }
                let ms = *ms.branches().first().unwrap_or(&&ms);
                assert_eq!(ms.get_leapk(), k);
            })
    }

    #[test]
    fn get_hashes() {
        gen_testcases()
            .into_iter()
            .for_each(|(ms, k, h, test_top_level)| {
                if !test_top_level {
                    return;
                }
                let ms = *ms.branches().first().unwrap_or(&&ms);
                let mut all: Vec<hash160::Hash> = k
                    .iter()
                    .map(|p| hash160::Hash::hash(&p.to_bytes()))
                    .collect();
                // In our test cases we always have plain keys going first
                all.extend(h);
                assert_eq!(ms.get_leapkh(), all);
            })
    }

    #[test]
    fn get_pubkey_and_hashes() {
        gen_testcases()
            .into_iter()
            .for_each(|(ms, k, h, test_top_level)| {
                if !test_top_level {
                    return;
                }
                let ms = *ms.branches().first().unwrap_or(&&ms);
                let r: Vec<PkPkh<bitcoin::PublicKey>> = if k.is_empty() {
                    h.into_iter().map(|h| PkPkh::HashedPubkey(h)).collect()
                } else {
                    k.into_iter().map(|k| PkPkh::PlainPubkey(k)).collect()
                };
                assert_eq!(ms.get_leapk_pkh(), r);
            })
    }

    #[test]
    fn find_keys() {
        gen_testcases().into_iter().for_each(|(ms, k, _, _)| {
            assert_eq!(ms.iter_pk().collect::<Vec<bitcoin::PublicKey>>(), k);
        })
    }

    #[test]
    fn find_hashes() {
        gen_testcases().into_iter().for_each(|(ms, k, h, _)| {
            let mut all: Vec<hash160::Hash> = k
                .iter()
                .map(|p| hash160::Hash::hash(&p.to_bytes()))
                .collect();
            // In our test cases we always have plain keys going first
            all.extend(h);
            assert_eq!(ms.iter_pkh().collect::<Vec<hash160::Hash>>(), all);
        })
    }

    #[test]
    fn find_pubkeys_and_hashes() {
        gen_testcases().into_iter().for_each(|(ms, k, h, _)| {
            let mut all: Vec<PkPkh<bitcoin::PublicKey>> =
                k.into_iter().map(|k| PkPkh::PlainPubkey(k)).collect();
            all.extend(h.into_iter().map(|h| PkPkh::HashedPubkey(h)));
            assert_eq!(
                ms.iter_pk_pkh().collect::<Vec<PkPkh<bitcoin::PublicKey>>>(),
                all
            );
        })
    }
}
