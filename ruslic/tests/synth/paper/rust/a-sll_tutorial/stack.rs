use russol_contracts::*;

pub struct List<T> {
    head: Link<T>,
}

type Link<T> = Option<Box<Node<T>>>;

struct Node<T> {
    elem: T,
    next: Link<T>,
}

#[extern_spec]
#[ensures(matches!(^x, None))]
#[ensures(*x === result)]
fn take<T>(x: &mut Option<T>) -> Option<T> { ruslik!() }

#[pure]
fn is_some<T>(opt: &Option<T>) -> bool { matches!(opt, Some(_)) }

impl<T> Node<T> {
    #[pure]
    #[trusted_ensures(result >= 1)]
    pub fn len_gt(&self) -> u16 {
        match &self.next {
            None => 1,
            Some(n) => 1 + n.len_gt(),
        }
    }
}

impl<T> List<T> {
    #[ensures(result.len() == 0)]
    pub fn new() -> Self {
      List { head: ::std::option::Option::None }
    }

    #[pure]
    #[trusted_ensures(result >= 0)]
    pub fn len(&self) -> u16 {
        match &self.head {
            None => 0,
            Some(n) => n.len_gt(),
        }
    }

    // #[ensures((^self).len() == self.len() + 1)]
    #[ensures(match &(^self).head {
        // Some(node) => node.elem === elem,
        Some(node) => node.next === self.head,
        None => false,
    })]
    pub fn push(&mut self, elem: T) {
      let result = take(&mut self.head);
      let bx = Node { elem, next: result };
      let _0 = Box::new(bx);
      let new = ::std::option::Option::Some(_0);
      self.head = new
    }

    // #[ensures(match (&result, &self.head) {
    //     (Some(v), Some(node)) => (^self).head === node.next && v === node.elem,
    //     (None, None) => true,
    //     _ => false,
    // })]
    #[ensures(match &self.head {
      Some(node) => (^self).head === node.next,
      None => true,
    })]
    pub fn pop(&mut self) -> Option<T> {
      let result = take(&mut self.head);
      match result {
        ::std::option::Option::None => ::std::option::Option::None,
        ::std::option::Option::Some(_0) => {
          let result = ::std::option::Option::Some(_0.elem);
          self.head = _0.next;
          result
        }
      }
    }

    // #[ensures(match (result, &self.head) {
    //     (Some(v), Some(node)) => v === node.elem,
    //     (None, None) => true,
    //     _ => false,
    // })]
    #[ensures(is_some(&self.head) == is_some(&result))]
    pub fn peek(&self) -> Option<&T> {
      match &self.head {
        ::std::option::Option::None => ::std::option::Option::None,
        ::std::option::Option::Some(_0) => ::std::option::Option::Some(&_0.elem),
      }
    }


    #[ensures(match (&(*self).head, result, &(^self).head) {
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
}

// impl<T> Drop for List<T> {
//     fn drop(&mut self) {
//         let mut cur_link = self.head.take();
//         while let Some(mut boxed_node) = cur_link {
//             cur_link = boxed_node.next.take();
//         }
//     }
// }

pub struct Iter<'a, T> {
    next: Option<&'a Node<T>>,
}

impl<'a, T> Iter<'a, T> {
    #[pure]
    #[trusted_ensures(result >= 0)]
    pub fn len(&self) -> u16 {
        match &self.next {
            None => 0,
            Some(n) => n.len_gt(),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[ensures(match self.len() {
        0 => (^self).len() == 0 && matches!(result, None),
        v => (^self).len() == v-1 && matches!(result, Some(_)),
    })]
    fn next(&mut self) -> Option<&'a T> {
      let (new, result) = match &mut self.next {
        ::std::option::Option::None => (::std::option::Option::None, ::std::option::Option::None),
        ::std::option::Option::Some(_0) => match &_0.next {
            ::std::option::Option::None => {
              let result = ::std::option::Option::Some(&_0.elem);
              (::std::option::Option::None, result)
            }
            ::std::option::Option::Some(_0_next_de) => {
              let result = ::std::option::Option::Some(&_0.elem);
              let new = ::std::option::Option::Some(&**_0_next_de);
              (new, result)
            }
          },
      };
      self.next = new;
      result
    }
}

// Should be solvable, but very slow:

pub struct IterMut<'a, T> {
    next: Option<&'a mut Node<T>>,
}

impl<'a, T> IterMut<'a, T> {
    #[pure]
    #[trusted_ensures(result >= 0)]
    pub fn len(&self) -> u16 {
        match &self.next {
            None => 0,
            Some(n) => n.len_gt(),
        }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    #[ensures(match (&self.next, result) {
        (None, None) => true,
        (Some(node), Some(result)) => (^*node).elem === ^result,
        _ => false,
    })]
    // #[ensures(match self.len() {
    //     0 => (^self).len() == 0,
    //     n => (^self).len() == n-1,
    // })]
    fn next(&mut self) -> Option<&'a mut T> {
      let result = take(&mut self.next);
      match result {
        ::std::option::Option::None => ::std::option::Option::None,
        ::std::option::Option::Some(_0) => ::std::option::Option::Some(&mut _0.elem),
      }
    }
}
