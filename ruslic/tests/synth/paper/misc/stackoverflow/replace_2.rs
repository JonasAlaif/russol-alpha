// https://stackoverflow.com/questions/28258548/cannot-move-out-of-borrowed-content-when-trying-to-transfer-ownership
use russol_contracts::*;

pub struct LinkedList {
    head: Option<Box<LinkedListNode>>,
}

pub struct LinkedListNode {
    next: Option<Box<LinkedListNode>>,
}

impl LinkedList {
    #[ensures(match &(^self).head {
        None => false,
        Some(lln) => lln.next === self.head
    })]
    pub fn prepend_value(&mut self) {
      let result = replace(&mut self.head, ::std::option::Option::None);
      let bx = LinkedListNode { next: result };
      let _0 = Box::new(bx);
      let new = ::std::option::Option::Some(_0);
      self.head = new
    }
}

#[extern_spec]
#[ensures(*dest === result)]
#[ensures(^dest === src)]
fn replace<T>(dest: &mut T, src: T) -> T { std::mem::replace(dest, src) }
