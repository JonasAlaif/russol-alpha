#![feature(box_patterns)]
use russol_contracts::*;

pub enum Tree {
    Node(i16, Box<Tree>, Box<Tree>),
    Empty,
}

impl Tree {
    #[pure]
    pub fn elems(&self) -> Set<i16> {
        if let Tree::Node(value, left, right) = self {
            left.elems() + right.elems() + set!{ value }
        } else { set!{} }
    }

    #[pure]
    pub fn rightmost(&self) -> i16 {
        match self {
            Tree::Empty => i16::MAX,
            Tree::Node(value, _, box Tree::Empty) => *value,
            Tree::Node(_, _, right) => right.rightmost(),
        }
    }

    #[pure]
    pub fn leftmost(&self) -> i16 {
        match self {
            Tree::Empty => i16::MIN,
            Tree::Node(value, box Tree::Empty, _) => *value,
            Tree::Node(_, left, _) => left.leftmost(),
        }
    }

    #[pure]
    pub fn bst_invariant(&self) -> bool {
        if let Tree::Node(value, left, right) = self {
            (if let Tree::Node(..) = &**left { *value >= left.rightmost() } else { true }) &&
            (if let Tree::Node(..) = &**right { *value <= right.leftmost() } else { true }) &&
            left.bst_invariant() && right.bst_invariant()
        } else { true }
    }

    // Requires branch abduction:
    // #[requires(self.bst_invariant())]
    // #[ensures((^self).bst_invariant())]
    // #[ensures((^self).elems()[&new_value])]
    // #[ensures((^self).elems() >= self.elems())]
    // pub fn insert(&mut self, new_value: i16) {
    //     if let Tree::Node(value, left, right) = self {
    //         match new_value.cmp(value) {
    //             Equal => (),
    //             Less => left.insert(new_value),
    //             Greater => right.insert(new_value),
    //         }
    //     } else {
    //         *self = Tree::Node(new_value, Box::new(Tree::Empty), Box::new(Tree::Empty))
    //     }
    // }

    #[requires(self.bst_invariant())]
    #[ensures((^self).bst_invariant())]
    #[requires(
        if let Tree::Node(_, left, right) = &*self {
            (if let Tree::Node(..) = &**left { ^result >= left.rightmost() } else { true }) &&
            (if let Tree::Node(..) = &**right { ^result <= right.leftmost() } else { true })
        } else { false }
    )]
    // Not required:
    // #[ensures(
    //     match (&^self, &*self) {
    //         (Tree::Node(fv, fl, fr), Tree::Node(ov, ol, or)) =>
    //             *fv == ^result && *ov == *result && fl === ol && fr === or,
    //         _ => false,
    //     }
    // )]
    pub fn get_root_value(&mut self) -> &mut i16 {
      match self {
        Tree::Node(_0, _, _) => _0,
        Tree::Empty => unreachable!(),
      }
    }
}
