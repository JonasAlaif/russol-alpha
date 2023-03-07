use russol_contracts::*;

enum Node {
    Nil,
    Cons { f: i32, next: Box<Node>, },
}

impl Node {
    #[pure]
    #[trusted_ensures(result >= 0)]
    fn len(&self) -> usize {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len(),
        }
    }
    #[pure]
    fn elems(&self) -> Set<i32> {
        match self {
            Node::Nil => set!{},
            Node::Cons { f, next } => next.elems() + set!{f},
        }
    }
}

#[ensures(result.len() == x.len())]
#[ensures(if result.len() == 0 { result.elems() == set!{} } else { result.elems() == set!{&0} })]
#[ensures(^x === ^result)]
fn zero(x: &mut Node) -> &mut Node {
  zero_4(x);
  x
}
#[helper] fn zero_4(x: &mut Node) {
  match x {
    Node::Nil => (),
    Node::Cons { f, next } => {
      zero_4(&mut **next);
      *f = 0 as i32
    }
  }
}
