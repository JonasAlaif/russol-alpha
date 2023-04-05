use russol_contracts::*;

struct List {
    head: Node,
}
enum Node {
    Nil,
    Cons { f: i32, next: Box<Node>, },
}

impl Node {
    #[pure]
    fn len(&self) -> usize {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len(),
        }
    }
    #[pure]
    fn sum(&self) -> i32 {
        match self {
            Node::Nil => 0,
            Node::Cons { f, next } => *f + next.sum(),
        }
    }
}

#[ensures((^x).head.len() == x.head.len())]
#[ensures((^x).head.sum() == 0)]
fn zero(x: &mut List) {
  zero_6(&mut x.head)
}
#[helper] fn zero_6(head: &mut Node) {
  match head {
    Node::Nil => (),
    Node::Cons { f, next } => {
      zero_6(&mut **next);
      *f = 0 as i32
    }
  }
}
