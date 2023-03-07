use russol_contracts::*;

pub enum List {
    Nil,
    Cons(u16, Box<List>),
}
use List::*;

impl List {
    #[pure]
    // #[trusted_ensures(result >= 0)]
    fn sum(&self) -> u16 {
        match self {
            Cons(a, l) => *a + l.sum(),
            Nil => 0,
        }
    }

    #[requires(self.sum() <= u16::MAX)]
    #[ensures(result == self.sum())]
    fn sum_x(&self) -> u16 {
      match self {
        List::Nil => 0 as u16,
        List::Cons(_0, _1) => {
          let de = *_0;
          let result = _1.sum_x();
          (de + result) as u16
        }
      }
    }

    #[requires(self.sum() > 0)]
    #[ensures((^self).sum() - self.sum() ==
        ^result.0 + (^result.1).sum() - *result.0 - (*result.1).sum())]
        // We need: `trusted_ensures(result >= 0)` on `sum` for this,
        // but trusted_ensures doesn't work well with `^self`.
    // #[ensures(*result.0 <= self.sum())]
    // #[ensures(result.1.sum() <= self.sum())]
    fn take_some_rest(&mut self) -> (&mut u16, &mut List) {
      match self {
        List::Nil => unreachable!(),
        List::Cons(_0, _1) => (_0, &mut **_1),
      }
    }
}
