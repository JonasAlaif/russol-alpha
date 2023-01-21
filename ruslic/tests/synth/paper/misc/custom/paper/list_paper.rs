use russol_contracts::*;

enum Node<T> {
    Nil,
    Cons { elem: T, next: Box<Node<T>> },
}

#[extern_spec]
#[ensures(*dest === result)]
#[ensures(^dest === src)]
fn replace<T>(dest: &mut T, src: T) -> T { std::mem::replace(dest, src) }

#[pure]
fn is_some<T>(o: &Option<T>) -> bool { matches!(o, Some(_)) }

impl<T> Node<T> {
    #[pure]
    fn len(&self) -> u16 {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len(),
        }
    }

    pub fn singleton(elem: T) -> Self {
      todo!()
      // let next = Box::new(Node::Nil);
      // Node::Cons { elem, next }
    }

    #[requires(self.len() > 0)]
    pub fn peek(&self) -> &T {
      match self {
        Node::Nil => unreachable!(),
        Node::Cons { elem, .. } => elem,
      }
    }

    #[ensures((^self).len() == self.len() + 1)]
    pub fn push_len(&mut self, elem: T) {
      match self {
        Node::Nil => {
          let next = Box::new(Node::Nil);
          let new = Node::Cons { elem, next };
          *self = new
        }
        Node::Cons { next, .. } => next.push_len(elem),
      }
    }

    #[ensures(match ^self {
        Node::Cons { ref next, .. } => **next === *self,
        Node::Nil => false,
    })]
    pub fn push(&mut self, elem: T) {
      let result = replace(self, Node::Nil);
      let next = Box::new(result);
      let new = Node::Cons { elem, next };
      *self = new
    }

    #[ensures(self.len() > 0 ==>
        (^self).len() == self.len()-1 && is_some(&result)
    )]
    pub fn pop(&mut self) -> Option<T> {
      let result = replace(self, Node::Nil);
      match result {
        Node::Nil => ::std::option::Option::None,
        Node::Cons { elem, next } => {
          *self = *next;
          ::std::option::Option::Some(elem)
        }
      }
    }

    #[ensures((^self).len() == self.len() + (^result).len())]
    pub fn peek_last(&mut self) -> &mut Self {
      match self {
        Node::Nil => self,
        Node::Cons { next, .. } => next.peek_last(),
      }
    }
}
