#![feature(box_patterns)]
use russol_contracts::*;

enum List<T> {
    Nil,
    Cons(Box<(T, List<T>)>),
}

impl<T> List<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            List::Nil => set!{},
            List::Cons(box (hd, tl)) => tl.elems() + set!{ hd },
        }
    }
    #[pure]
    #[trusted_ensures(result >= 0 && result <= u16::MAX)]
    fn len(&self) -> u16 {
        match self {
            List::Nil => 0,
            List::Cons(box (_, tl)) => 1 + tl.len(),
        }
    }
}

// multilist
impl<T> List<List<T>> {
    #[pure]
    fn elems_list(&self) -> Set<T> {
        match self {
            List::Nil => set!{},
            List::Cons(box (hd, tl)) => hd.elems() + tl.elems_list(),
        }
    }
    #[pure]
    #[trusted_ensures(result >= 0 && result <= u16::MAX)]
    fn mlen(&self) -> u16 {
        match self {
            List::Nil => 0,
            List::Cons(box (hd, tl)) => hd.len() + tl.mlen(),
        }
    }

    #[ensures(result.elems() == self.elems_list())]
    #[params("--closeWhileAbduce=false")]
    fn flatten(self) -> List<T> {
      match self {
        List::Nil => List::Nil,
        List::Cons(_0) => Self::flatten_7(_0.0, _0.1),
      }
    }
    #[helper] fn flatten_7(_0: List<T>, _1: List<List<T>>) -> List<T> {
      match _0 {
        List::Nil => _1.flatten(),
        List::Cons(_0) => {
          let result = Self::flatten_7(_0.1, _1);
          let bx = (_0.0, result);
          let _0 = Box::new(bx);
          List::Cons(_0)
        }
      }
    }

    // FAILURE:
    // Doesn't work due to having `hd.tail.len() + tl.mlen()` in a variable,
    // but not being able to construct `1 + hd.tail.len() + tl.mlen()`
    // #[requires(self.mlen() <= u16::MAX)]
    // #[ensures(result == self.mlen())]
    // fn multilist_length(&self) -> u16 {
    //     match self {
    //         List::Nil => 0,
    //         List::Cons(box (hd, tl)) => hd.len() + tl.mlen(),
    //     }
    // }
}
