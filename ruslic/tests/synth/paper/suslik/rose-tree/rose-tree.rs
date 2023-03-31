#![feature(box_patterns)]
use russol_contracts::*;

enum RoseTree<T> {
    Nil,
    Cons { elem: T, next: List<RoseTree<T>> },
}
enum List<T> {
    Nil,
    Cons(Box<(T, List<T>)>),
}

impl<T> List<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            List::Nil => set!{},
            List::Cons(box (hd, tl)) => tl.elems() + set!{ hd },
        }
    }
}
impl<T> List<RoseTree<T>> {
    #[pure]
    fn elems_tree(&self) -> Set<T> {
        match self {
            List::Nil => set!{},
            List::Cons(box (hd, tl)) => hd.elems() + tl.elems_tree(),
        }
    }
}

// rose-tree
impl<T> RoseTree<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            RoseTree::Nil => set!{},
            RoseTree::Cons { elem, next } => next.elems_tree() + set!{ elem },
        }
    }

    #[ensures(result.elems() == self.elems())]
    fn copy(&self) -> Self where T: Copy {
        ruslik!()
    }

    #[ensures(result.elems() == self.elems())]
    #[params("--closeWhileAbduce=false")]
    fn flatten(self) -> List<T> {
        ruslik!()
    }
}
