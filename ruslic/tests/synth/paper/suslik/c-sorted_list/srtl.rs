#![feature(box_patterns)]
use russol_contracts::*;

enum Node<T> {
    Nil,
    Cons { elem: T, next: Box<Node<T>> },
}

impl<T> Node<T> {
    #[pure]
    fn len(&self) -> u16 {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len(),
        }
    }
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            Node::Nil => set![],
            Node::Cons { elem, next } => next.elems() + set!{ elem },
        }
    }
}

impl Node<u16> {
    #[pure]
    fn is_sorted(&self) -> bool {
        match self {
            Node::Nil | Node::Cons { next: box Node::Nil, .. } => true,
            Node::Cons { elem, next: box next@Node::Cons { elem: e, .. } } =>
                *elem <= *e && next.is_sorted(),
        }
    }

    #[extern_spec]
    #[requires(self.is_sorted())]
    #[ensures(result.is_sorted())]
    #[ensures(result.elems() == self.elems() + set!{ &v })]
    fn srtl_insert(self, v: u16) -> Self { todo!() }

    #[ensures(result.elems() == self.elems())]
    #[ensures(result.is_sorted())]
    fn insertion_sort(&self) -> Self {
      match self {
        Node::Nil => Node::Nil,
        Node::Cons { elem, next } => {
          let de = *elem;
          let result = next.insertion_sort();
          result.srtl_insert(de)
        }
      }
    }

    // Branch abduction:
    // #[requires(self.is_sorted() && other.is_sorted())]
    // #[ensures(result.elems() == self.elems() + other.elems())]
    // #[ensures(result.is_sorted())]
    // fn srtl_merge(self, other: Self) -> Self {
    //     todo!()
    // }

    // Doesn't work properly since it would require `interval`s
    #[requires(if let Node::Cons { elem, .. } = self { v <= elem } else { true })]
    #[ensures(result.len() == self.len() + 1)]
    fn srtl_prepend(self, v: u16) -> Self {
      let next = Box::new(self);
      Node::Cons { elem: v as u16, next }
    }

    // Requires intervals:
    // #[pure]
    // fn is_sorted_rev(&self) -> bool {
    //     match self {
    //         Node::Nil => true,
    //         Node::Cons { next: box next@Node::Nil, .. } => next.is_sorted_rev(),
    //         Node::Cons { elem, next: box next@Node::Cons { elem: e, .. } } =>
    //             *elem >= *e && next.is_sorted_rev(),
    //     }
    // }
    // #[requires(self.is_sorted())]
    // #[ensures(result.is_sorted_rev())]
    // #[ensures(result.elems() == self.elems())]
    // fn srtl_rev(self) -> Self {
    //     todo!()
    // }
}
