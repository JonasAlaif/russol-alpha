use russol_contracts::*;

enum Node<T> {
    Nil,
    Cons { elem: T, next: Box<Node<T>> },
}

impl<T> Node<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            Node::Nil => set!{},
            Node::Cons { elem, next } => next.elems() + set!{ elem },
        }
    }

    #[extern_spec]
    #[ensures(result.elems() == self.elems() + other.elems())]
    fn append(self, other: Self, token: Token) -> Self {
        todo!()
    }
}

enum Tree<T> {
    Leaf,
    Node { elem: T, left: Box<Tree<T>>, right: Box<Tree<T>>, },
}

impl<T> Tree<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            Tree::Leaf => set!{},
            Tree::Node { elem, left, right } => left.elems() + right.elems() + set!{ elem },
        }
    }

    #[ensures(result.elems() == self.elems())]
    fn tree_copy(&self) -> Self where T: Copy {
      match self {
        Tree::Leaf => Tree::Leaf,
        Tree::Node { elem, left, right } => {
          let de = *elem;
          let result_1 = left.tree_copy();
          let result_2 = right.tree_copy();
          let right = Box::new(result_2);
          let left = Box::new(result_1);
          Tree::Node { elem: de, left, right }
        }
      }
    }

    #[ensures((^l).elems() == self.elems() + l.elems())]
    #[params("--closeWhileAbduce=false")]
    fn tree_flatten_acc(self, l: &mut Node<T>) {
      match l {
        Node::Nil => {
          let new = self.tree_flatten_acc_7();
          *l = new
        }
        Node::Cons { next, .. } => self.tree_flatten_acc(&mut **next), // <- TODO: investigate why the args here get swapped
      }
    }
    #[helper] fn tree_flatten_acc_7(self) -> Node<T> {
      match self {
        Tree::Leaf => Node::Nil,
        Tree::Node { elem, left, right } => {
          let new = left.tree_flatten_acc_7();
          Self::tree_flatten_acc_18(elem, *right, new)
        }
      }
    }
    #[helper] fn tree_flatten_acc_18(elem: T, bx: Tree<T>, new: Node<T>) -> Node<T> {
      match new {
        Node::Nil => {
          let new = bx.tree_flatten_acc_7();
          let next = Box::new(new);
          Node::Cons { elem, next }
        }
        Node::Cons { elem: elem_new, next } => {
          let new = Self::tree_flatten_acc_18(elem_new, bx, *next);
          let next = Box::new(new);
          Node::Cons { elem, next }
        }
      }
    }

    // fn tree_dll{,_linear}

    #[ensures(result.elems() == self.elems())]
    #[params("--closeWhileAbduce=false")]
    fn tree_flatten_helper(self, token: Token) -> Node<T> {
      match self {
        Tree::Leaf => Node::Nil,
        Tree::Node { elem, left, right } => {
          let result_1 = left.tree_flatten_helper(token);
          let result_2 = right.tree_flatten_helper(token);
          let result = result_1.append(result_2, token);
          let next = Box::new(result);
          Node::Cons { elem, next }
        }
      }
    }

    #[ensures(result.elems() == self.elems())]
    #[params("--closeWhileAbduce=false")]
    fn tree_flatten(self) -> Node<T> {
      match self {
        Tree::Leaf => Node::Nil,
        Tree::Node { elem, left, right } => {
          let result = left.tree_flatten();
          Self::tree_flatten_12(elem, *right, result)
        }
      }
    }
    #[helper] fn tree_flatten_12(elem: T, bx: Tree<T>, result: Node<T>) -> Node<T> {
      match result {
        Node::Nil => {
          let result = bx.tree_flatten();
          let next = Box::new(result);
          Node::Cons { elem, next }
        }
        Node::Cons { elem: elem_result, next } => {
          let result = Self::tree_flatten_12(elem_result, bx, *next);
          let next = Box::new(result);
          Node::Cons { elem, next }
        }
      }
    }

    // fn free{,2}

    #[pure]
    #[trusted_ensures(result <= u16::MAX)]
    fn size(&self) -> u16 {
        match self {
            Tree::Leaf => 0,
            Tree::Node { left, right, .. } => left.size() + right.size() + 1,
        }
    }

    #[ensures(result == self.size())]
    fn tree_size(&self) -> u16 {
      match self {
        Tree::Leaf => 0 as u16,
        Tree::Node { left, right, .. } => {
          let result_1 = left.tree_size();
          let result_2 = right.tree_size();
          (result_1 + result_2 + 1) as u16
        }
      }
    }
}

#[derive(Copy, Clone)]
struct Token<'a>(&'a Token<'a>);
