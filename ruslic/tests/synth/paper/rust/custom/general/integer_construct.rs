use russol_contracts::*;

enum Node {
    Nil,
    Cons { f: i32, next: Box<Node>, },
}

impl Node {
    #[pure]
    #[trusted_ensures(0 <= result)]
    fn len_te(&self) -> u16 {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len_te(),
        }
    }
}

#[requires(x.len_te() <= u16::MAX)]
#[ensures((^x).len_te() == x.len_te())]
#[ensures(result == x.len_te())]
fn len(x: &mut Node) -> u16 {
  match x {
    Node::Nil => 0 as u16,
    Node::Cons { next, .. } => {
      let result = len(&mut **next);
      (result + 1) as u16
    }
  }
}
