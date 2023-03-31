use russol_contracts::*;

enum Node1<T> {
    Nil,
    Cons { node: Box<(T, Node1<T>)> },
}

#[extern_spec]
#[ensures(*dest === result)]
#[ensures(^dest === src)]
fn replace<T>(dest: &mut T, src: T) -> T { std::mem::replace(dest, src) }

#[pure]
fn is_some<T>(o: &Option<T>) -> bool { matches!(o, Some(_)) }

impl<T> Node1<T> {
    #[pure]
    fn len(&self) -> u16 {
        match self {
            Node1::Nil => 0,
            Node1::Cons { node } => 1 + node.1.len(),
        }
    }

    pub fn singleton(elem: T) -> Self {
      let bx = (elem, Node1::Nil);
      let node = Box::new(bx);
      Node1::Cons { node }
    }

    #[requires(self.len() > 0)]
    pub fn peek(&self) -> &T {
      match self {
        Node1::Nil => unreachable!(),
        Node1::Cons { node } => &node.0,
      }
    }

    #[ensures((^self).len() == self.len() + 1)]
    pub fn push_len(&mut self, elem: T) {
      match self {
        Node1::Nil => {
          let bx = (elem, Node1::Nil);
          let node = Box::new(bx);
          let new = Node1::Cons { node };
          *self = new
        }
        Node1::Cons { node } => node.1.push_len(elem),
      }
    }

    #[ensures(match ^self {
        Node1::Cons { ref node } => node.1 === *self,
        Node1::Nil => false,
    })]
    pub fn push(&mut self, elem: T) {
      let result = replace(self, Node1::Nil);
      let bx = (elem, result);
      let node = Box::new(bx);
      let new = Node1::Cons { node };
      *self = new
    }

    #[ensures(self.len() > 0 ==>
        (^self).len() == self.len()-1 && is_some(&result)
    )]
    pub fn pop(&mut self) -> Option<T> {
      let result = replace(self, Node1::Nil);
      match result {
        Node1::Nil => ::std::option::Option::None,
        Node1::Cons { node } => {
          *self = node.1;
          ::std::option::Option::Some(node.0)
        }
      }
    }

    #[ensures((^self).len() == self.len() + (^result).len())]
    pub fn peek_last(&mut self) -> &mut Self {
      match self {
        Node1::Nil => self,
        Node1::Cons { node } => node.1.peek_last(),
      }
    }
}
