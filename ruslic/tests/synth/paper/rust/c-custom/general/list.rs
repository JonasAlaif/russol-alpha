#![feature(box_patterns)]
use russol_contracts::*;

struct Tuple<'b, 'c>(&'b mut i32, &'c mut Node<i32>);
enum Node<T> {
    Nil,
    Cons { elem: T, next: Box<Node<T>> }
}

impl<T> Node<T> {
    #[pure]
    fn len(&self) -> usize {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len(),
        }
    }
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            Node::Nil => set!{},
            Node::Cons { elem, next } => next.elems() + set!{elem},
        }
    }
    #[ensures(result.len() == self.len())]
    fn modify_elems(&mut self) -> Node<&mut T> {
        match self {
          Node::Nil => Node::Nil,
          Node::Cons { elem, next } => {
            let result = next.modify_elems();
            let next = Box::new(result);
            Node::Cons { elem, next }
          }
        }
    }
}

impl Node<i32> {
    #[pure]
    fn elems_eq(&self) -> bool {
        match self {
            Node::Cons { elem, next: box next@Node::Cons { elem: fnxt, .. }, .. } =>
                *elem == *fnxt && next.elems_eq(),
            _ => true,
        }
    }

    #[requires(i.len() >= 2)]
    #[ensures((^i).len() == 2 + (^result.1).len())]
    #[ensures((^i).elems() == match &i {
        Node::Cons { elem, .. } => Set::new(&[elem, &^result.0]) + (^result.1).elems(),
        _ => set!{},
    })]
    fn reborrow_head_and_tail_2<'a: 'b + 'c, 'b, 'c>(i: &'a mut &mut Self) -> Tuple<'b, 'c> {
        match &mut **i {
            Node::Nil => unreachable!(),
            Node::Cons { next, .. } => match &mut **next {
                Node::Nil => unreachable!(),
                Node::Cons { elem, next } => Tuple(elem, &mut **next),
            },
        }
    }

    #[requires(self.len() >= 2)]
    #[ensures(self.len() - 2 == result.len())]
    fn tail2(self) -> Self {
        match self {
            Node::Nil => unreachable!(),
            Node::Cons { next, .. } => match *next {
                Node::Nil => unreachable!(),
                Node::Cons { next, .. } => *next,
            },
        }
    }
}

struct List {
    head: Node<i32>,
}
impl List {
    #[ensures((^self).head.len() == (*self).head.len() + tl.head.len())]
    #[ensures((^self).head.elems() == (*self).head.elems() + tl.head.elems())]
    fn append(&mut self, tl: Self) {
        Self::append_8(&mut self.head, tl.head)
    }
    #[helper] fn append_8(head_self: &mut Node<i32>, head_tl: Node<i32>) {
        match head_self {
            Node::Nil => *head_self = head_tl,
            Node::Cons { next, .. } => Self::append_8(&mut **next, head_tl),
        }
    }

    #[ensures((*self).head.len() == (^self).head.len())]
    #[ensures((^self).head.elems_eq() == true)]
    fn lstset(&mut self) {
      let new = Self::lstset_7(&mut self.head);
      self.head = new
    }
    #[helper] fn lstset_7(head: &mut Node<i32>) -> Node<i32> {
      match head {
        Node::Nil => Node::Nil,
        Node::Cons { elem, next } => {
          let de = *elem;
          let new = Self::lstset_7(&mut **next);
          match new {
            Node::Nil => {
              let next = Box::new(Node::Nil);
              Node::Cons { elem: de as i32, next }
            }
            Node::Cons { elem, next } => {
              let bx = Node::Cons { elem: elem as i32, next };
              let next = Box::new(bx);
              Node::Cons { elem: elem as i32, next }
            }
          }
        }
      }
    }

    #[ensures(self.head.len() == result.head.len())]
    #[ensures(self.head.elems() == result.head.elems())]
    fn duplicate(&self) -> List {
        Self::new_list_6(&self.head)
    }
    #[helper] fn new_list_6(head: &Node<i32>) -> List {
        match head {
            Node::Nil => List { head: Node::Nil },
            Node::Cons { elem, next } => {
                let de = *elem;
                let result = Self::new_list_6(&**next);
                let next = Box::new(result.head);
                let head = Node::Cons { elem: de as i32, next };
                List { head }
            }
        }
    }
}
