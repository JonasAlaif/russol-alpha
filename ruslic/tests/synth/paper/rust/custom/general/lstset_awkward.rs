use russol_contracts::*;

struct Node1 {
    elem: i32,
    next: Option<Box<Node1>>,
}
pub struct List { head: Option<Box<Node1>>, }

impl Node1 {
    #[pure]
    fn eq(&self, val: i32) -> bool {
        self.elem == val &&
        match &self.next {
            None => true,
            Some(next) => next.eq(val),
        }
    }
    #[pure]
    fn len(&self) -> i32 {
        1 +
        match &self.next {
            None => 0,
            Some(next) => next.len(),
        }
    }
    #[pure]
    fn sum(&self) -> i32 {
        self.elem +
        match &self.next {
            None => 0,
            Some(next) => next.sum(),
        }
    }
}

#[ensures(match (&list.head, &(^list).head)  {
    (None, None) => true,
    (Some(list), Some(fut)) => list.len() == fut.len(),
    _ => false,
})]
#[ensures(match &(^list).head {
    None => true,
    Some(list) => list.sum() == list.len() * val
})]
pub fn listset(list: &mut List, val: i32) {
  listset_6(val, &mut list.head)
}
#[helper] fn listset_6(val: i32, head: &mut std::option::Option<std::boxed::Box<Node1>>) {
  match head {
    ::std::option::Option::None => (),
    ::std::option::Option::Some(_0) => {
      listset_6(val, &mut _0.next);
      _0.elem = val as i32
    }
  }
}
