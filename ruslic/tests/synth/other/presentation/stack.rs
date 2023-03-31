use russol_contracts::*;

pub struct ListP<T> {
    head: Link<T>,
}

type Link<T> = Option<Box<NodeP<T>>>;

struct NodeP<T> {
    elem: T,
    next: Link<T>,
}

#[extern_spec]
#[ensures(result === *x)]
#[ensures(matches!(^x, None))]
fn take<T>(x: &mut Option<T>) -> Option<T> { ruslik!() }

impl<T> NodeP<T> {
    #[pure]
    pub fn len(&self) -> i32 {
        match &self.next {
            None => 1,
            Some(n) => 1 + n.len(),
        }
    }
    #[pure]
    pub fn elems(&self) -> Set<T> {
        match &self.next {
            None => set!{&self.elem},
            Some(n) => n.elems() + set!{&self.elem},
        }
    }
}

impl<T> ListP<T> {
    ///
    /// Example 1 [Simple owned]
    /// 
    #[ensures(matches!(result.head, Some(_)))]
    pub fn single(elem: T) -> Self {
      let bx = NodeP { elem, next: ::std::option::Option::None };
      let _0 = Box::new(bx);
      let head = ::std::option::Option::Some(_0);
      ListP { head }
    }

    ///
    /// Example 2 [Borrows/Measures]
    /// 
    #[ensures((^self).len() == self.len() + tail.len())]
    pub fn append(&mut self, tail: Self) {
      Self::append_8(&mut self.head, tail.head)
    }
    #[helper] fn append_8(head_self: &mut std::option::Option<std::boxed::Box<NodeP<T>>>, head_tail: std::option::Option<std::boxed::Box<NodeP<T>>>) {
      match head_self {
        ::std::option::Option::None => *head_self = head_tail,
        ::std::option::Option::Some(_0) => Self::append_8(&mut _0.next, head_tail),
      }
    }

    #[pure]
    pub fn len(&self) -> i32 {
        match &self.head {
            None => 0,
            Some(n) => n.len(),
        }
    }
    #[pure]
    pub fn elems(&self) -> Set<T> {
        match &self.head {
            None => set!{},
            Some(n) => n.elems(),
        }
    }

    ///
    /// Example 3 [External fns]
    /// 
    #[ensures((^self).len() == self.len() + 1)]
    #[ensures(match &(^self).head {
        Some(node) => node.elem === &elem,
        None => false,
    })]
    pub fn push(&mut self, elem: T) {
      let result = take(&mut self.head);
      let bx = NodeP { elem, next: result };
      let _0 = Box::new(bx);
      let new = ::std::option::Option::Some(_0);
      self.head = new
    }

    ///
    /// Example 4 [Reborrowing]
    /// 
    #[ensures(match (&self.head, result, &(^self).head) {
        (Some(node), Some(v), Some(fut)) =>
            node.elem === *v && ^v === fut.elem,
        (None, None, None) => true,
        _ => false,
    })]
    pub fn peek_mut(&mut self) -> Option<&mut T> {
      match &mut self.head {
        ::std::option::Option::None => ::std::option::Option::None,
        ::std::option::Option::Some(_0) => ::std::option::Option::Some(&mut _0.elem),
      }
    }

    #[ensures((^x).elems() >= x.elems())]
    pub fn peek_last<'a>(x: &'a mut &mut Self) -> &'a mut Link<T> {
      let result = take(&mut x.head);
      x.head = result;
      &mut x.head
    }
}
