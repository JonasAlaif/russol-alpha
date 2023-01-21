#![feature(box_patterns)]
use russol_contracts::*;

enum Node<T> {
    Nil,
    Cons { f: T, next: Box<Node<T>> },
}

impl Node<i32> {
    #[pure]
    fn len(&self) -> u16 {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len(),
        }
    }
    #[pure]
    fn elems(&self) -> Set<i32> {
        match self {
            Node::Nil => set![],
            Node::Cons { f, next } => set![f] + next.elems(),
        }
    }
    #[pure]
    fn is_sorted(&self) -> bool {
        match self {
            Node::Nil => true,
            Node::Cons { f, next } => next.is_sorted() &&
                if let box Node::Cons { f: f_n, .. } = next { *f <= *f_n } else { true },
        }
    }
}

/// Unsupported
#[requires(list.is_sorted())]
#[ensures((^list).is_sorted())]
// #[ensures(match (&*list, &^list) {
//     (Node::Nil, Node::Cons { f, next: box Node::Nil }) => f === v,
//     (Node::Cons { f, .. }, Node::Cons { f: ff, .. }) => if *f < v { f === ff } else { v === *ff },
//     _ => false
// })]
#[ensures((^list).len() == list.len() + 1)]
fn sorted_insert(list: &mut Node<i32>, v: i32) {
    ruslik!()
}
