use russol_contracts::*;

enum List<T> {
    Nil,
    Cons { elem: T, next: Box<List<T>> },
}

impl<T> List<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            List::Nil => set!{},
            List::Cons { elem, next } => next.elems() + set!{elem},
        }
    }
}

enum Tree<T> {
    Leaf,
    Node { f: T, left: Box<Tree<T>>, right: Box<Tree<T>>, },
}

impl<T> Tree<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            Tree::Leaf => set!{},
            Tree::Node { f, left, right } => left.elems() + right.elems() + set!{f},
        }
    }

    #[ensures(result.elems() == self.elems())]
    #[params("--closeWhileAbduce=false")]
    #[params("--memo=false")]
    fn to_list(self) -> List<T> {
      match self {
        Tree::Leaf => List::Nil,
        Tree::Node { f, left, right } => {
          let result = left.to_list();
          Self::to_list_12(f, *right, result)
        }
      }
    }
    #[helper] fn to_list_12(f: T, bx: Tree<T>, result: List<T>) -> List<T> {
      match result {
        List::Nil => {
          let result = bx.to_list();
          let next = Box::new(result);
          List::Cons { elem: f, next }
        }
        List::Cons { elem, next } => {
          let result = Self::to_list_12(elem, bx, *next);
          let next = Box::new(result);
          List::Cons { elem: f, next }
        }
      }
    }
}
