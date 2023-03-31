use russol_contracts::*;

enum Node<T> {
    Nil,
    Cons { f: T, next: Box<Node<T>>, },
}

impl<T> Node<T> {
    #[pure]
    #[trusted_ensures(result >= 0)]
    fn len_te(&self) -> usize {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len_te(),
        }
    }
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            Node::Nil => set!{},
            Node::Cons { f, next } => next.elems() + set!{f},
        }
    }
}

#[ensures(result.len_te() == x.len_te())]
#[ensures(if result.len_te() == 0 { result.elems() == set!{} } else { result.elems() == set!{&0} })]
#[ensures(^x === ^result)]
fn zero(x: &mut Node<i32>) -> &mut Node<i32> {
  zero_4(x);
  x
}
#[helper] fn zero_4(x: &mut Node<i32>) {
  match x {
    Node::Nil => (),
    Node::Cons { f, next } => {
      zero_4(&mut **next);
      *f = 0 as i32
    }
  }
}
