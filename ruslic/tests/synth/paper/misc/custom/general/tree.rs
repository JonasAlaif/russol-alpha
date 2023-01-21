use russol_contracts::*;


enum Tree {
    Leaf,
    Node { f: i32, left: Box<Tree>, right: Box<Tree>, },
}

impl Tree {
    #[pure]
    fn size(&self) -> usize {
        match self {
            Tree::Leaf => 0,
            Tree::Node { left, right, .. } => 1 + left.size() + right.size(),
        }
    }

    #[ensures(result.size() == self.size())]
    fn duplicate(&self) -> Self {
      match self {
        Tree::Leaf => Tree::Leaf,
        Tree::Node { f, left, right } => {
          let de = *f;
          let result_1 = left.duplicate();
          let result_2 = right.duplicate();
          let right = Box::new(result_2);
          let left = Box::new(result_1);
          Tree::Node { f: de as i32, left, right }
        }
      }
    }
}
