use russol_contracts::*;

enum Node<T> {
    Nil,
    Cons { node: Box<(T, Node<T>)> },
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
            Node::Cons { node } => 1 + node.1.len(),
        }
    }

    pub fn singleton(elem: T) -> Self {
      let bx = (elem, Node::Nil);
      let node = Box::new(bx);
      Node::Cons { node }
    }

    #[requires(self.len() > 0)]
    pub fn peek(&self) -> &T {
      match self {
        Node::Nil => unreachable!(),
        Node::Cons { node } => &node.0,
      }
    }

    #[ensures((^self).len() == self.len() + 1)]
    pub fn push_len(&mut self, elem: T) {
      match self {
        Node::Nil => {
          let bx = (elem, Node::Nil);
          let node = Box::new(bx);
          let new = Node::Cons { node };
          *self = new
        }
        Node::Cons { node } => node.1.push_len(elem),
      }
    }

    #[ensures(match ^self {
        Node::Cons { ref node } => node.1 === *self,
        Node::Nil => false,
    })]
    pub fn push(&mut self, elem: T) {
      let result = replace(self, Node::Nil);
      let bx = (elem, result);
      let node = Box::new(bx);
      let new = Node::Cons { node };
      *self = new
    }

    #[ensures(self.len() > 0 ==>
        (^self).len() == self.len()-1 && is_some(&result)
    )]
    pub fn pop(&mut self) -> Option<T> {
      let result = replace(self, Node::Nil);
      match result {
        Node::Nil => ::std::option::Option::None,
        Node::Cons { node } => {
          *self = node.1;
          ::std::option::Option::Some(node.0)
        }
      }
    }

    #[ensures((^self).len() == self.len() + (^result).len())]
    pub fn peek_last(&mut self) -> &mut Self {
      match self {
        Node::Nil => self,
        Node::Cons { node } => node.1.peek_last(),
      }
    }
}
