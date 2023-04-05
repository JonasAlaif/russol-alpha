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
    // #[pure]
    // fn count(&self, v: i32) -> u16 {
    //     match self {
    //         Node::Nil => 0,
    //         Node::Cons { f, next } => next.count(v) + if *f == v { 1 } else { 0 },
    //     }
    // }
    // #[pure]
    // fn is_upper_bound(&self, v: i32) -> bool {
    //     match self {
    //         Node::Nil => true,
    //         Node::Cons { f, next } => next.is_upper_bound(v) && v >= *f,
    //     }
    // }
    // #[pure]
    // fn count_unique(&self) -> u16 {
    //     match self {
    //         Node::Nil => 0,
    //         Node::Cons { f, next } => next.count_unique() + if next.count(*f) == 0 { 1 } else { 0 },
    //     }
    // }

    #[ensures(result.len() == self.len() + x2.len())]
    fn sll_append_copy(&self, x2: &Self) -> Self where T: Copy {
      match x2 {
        Node::Nil => self.sll_append_copy_7(),
        Node::Cons { elem, next } => {
          let de = *elem;
          let result = self.sll_append_copy(&**next);
          let next = Box::new(result);
          Node::Cons { elem: de, next }
        }
      }
    }
    #[helper] fn sll_append_copy_7(&self) -> Node<T> {
      match self {
        Node::Nil => Node::Nil,
        Node::Cons { elem, next } => {
          let de = *elem;
          let result = next.sll_append_copy_7();
          let next = Box::new(result);
          Node::Cons { elem: de, next }
        }
      }
    }

    #[ensures((^self).len() == (*self).len() + tl.len())]
    fn sll_append(&mut self, tl: Self) {
      match self {
        Node::Nil => *self = tl,
        Node::Cons { next, .. } => next.sll_append(tl),
      }
    }

    #[ensures(*self === result)]
    fn sll_copy(&self) -> Self where T: Copy {
      match self {
        Node::Nil => Node::Nil,
        Node::Cons { elem, next } => {
          let de = *elem;
          let result = next.sll_copy();
          let next = Box::new(result);
          Node::Cons { elem: de, next }
        }
      }
    }

    // #[ensures(result.elems() == self.elems() - set!{ &v })]
    // fn sll_delete_all(self, v: T) -> Self where T: Eq {
    //     ruslik!()
    // }

    // fn sll_diff(x: &Node<i32>, y: &Node<i32>) -> Node<i32> {
    //     ruslik!()
    // }

    // sll_free{,2}

    #[ensures((^self).len() == self.len())]
    #[ensures((^self).elems() <= set!{ &v })]
    fn sll_init(&mut self, v: T) where T: Copy {
      let new = self.sll_init_3(v);
      *self = new
    }
    #[helper] fn sll_init_3(&mut self, v: T) -> Node<T> {
      match self {
        Node::Nil => Node::Nil,
        Node::Cons { next, .. } => {
          let new = next.sll_init_3(v);
          let next = Box::new(new);
          Node::Cons { elem: v, next }
        }
      }
    }

    // fn sll_intersect(x: &Node<i32>, y: &Node<i32>) -> Node<i32> {
    //     ruslik!()
    // }

    #[requires(self.len() <= u16::MAX)]
    #[ensures(result == self.len())]
    fn sll_len(&self) -> u16 {
      match self {
        Node::Nil => 0 as u16,
        Node::Cons { next, .. } => {
          let result = next.sll_len();
          (result + 1) as u16
        }
      }
    }

    // #[ensures((^x).is_upper_bound(result))]
    // #[ensures((^x).len() == 0 || (^x).count(result) >= 1)]
    // fn sll_{max,min}(&self) -> &T where T: Ord {
    //     ruslik!()
    // }

    // TODO: uncommenting this makes it a lot faster, change that:
    // #[params("--closeWhileAbduce=false")]
    #[ensures((^self).len() == (*self).len() + y.len() + z.len())]
    fn sll_append3(&mut self, y: Self, z: Self) {
      match self {
        Node::Nil => {
          let new = Self::sll_append3_7(y, z);
          *self = new
        }
        Node::Cons { next, .. } => next.sll_append3(y, z),
      }
    }
    #[helper] fn sll_append3_7(y: Node<T>, z: Node<T>) -> Node<T> {
      match y {
        Node::Nil => z,
        Node::Cons { elem, next } => {
          let new = Self::sll_append3_7(z, *next);
          let next = Box::new(new);
          Node::Cons { elem, next }
        }
      }
    }
    
    fn sll_singleton(elem: T) -> Self {
      let next = Box::new(Node::Nil);
      Node::Cons { elem, next }
    }

    // fn sll_union(...) -> Self {
    //     ruslik!()
    // }

    // #[ensures(result.count_unique() == result.len())]
    // #[ensures(x.count_unique() == result.len())]
    // fn sll_unique(x: Node<i32>, ghost: u16) -> Node<i32> {
    //     ruslik!()
    // }
}
