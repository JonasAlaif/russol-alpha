#![feature(box_patterns)]

use russol_contracts::*;

enum Tree<T> {
    Leaf,
    Node { f: T, left: Box<Tree<T>>, right: Box<Tree<T>>, },
}

impl<T> Tree<T> {
    #[pure]
    fn size(&self) -> usize {
        match self {
            Tree::Leaf => 0,
            Tree::Node { left, right, .. } => 1 + left.size() + right.size(),
        }
    }
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            Tree::Leaf => set!{},
            Tree::Node { f, left, right } => left.elems() + right.elems() + set!{f},
        }
    }
}
impl Tree<i32> {
    #[pure]
    fn ordered(&self) -> bool {
        match self {
            Tree::Leaf => true,
            Tree::Node { f, left, right } => {
                let lord = if let box Tree::Node { f: f_l, .. } = left { *f_l <= *f } else { true };
                let rord = if let box Tree::Node { f: f_r, .. } = right { *f <= *f_r } else { true };
                lord && rord && left.ordered() && right.ordered()
            },
        }
    }

    #[ensures(match (&self, &^self) {
        (Tree::Leaf, _) => (^self).elems() == set!{&v},
        (Tree::Node { f, left, right },
         Tree::Node { left: l, right: r, .. }) => {
            if v < *f {
                l.elems() == set!{&v} + left.elems()
            } else if *f < v {
                r.elems() == set!{&v} + right.elems()
            } else { true }
        }
        _ => false
    })]
    #[ensures((^self).elems() == set!{&v} + self.elems())]
    fn insert(&mut self, v: i32) {
      match self {
        Tree::Leaf => {
          let right = Box::new(Tree::Leaf);
          let left = Box::new(Tree::Leaf);
          let new = Tree::Node { f: v as i32, left, right };
          *self = new
        }
        Tree::Node { f, left, right } => {
          let de = *f;
          if v < de { left.insert(v) } else if de < v { right.insert(v) }
        }
      }
    }
}
