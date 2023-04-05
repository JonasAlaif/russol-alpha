use russol_contracts::*;

struct BorrowAndValue<'a, T> {
    borrow: &'a mut T,
    value: T,
}

impl<'a, T> BorrowAndValue<'a, T> {
    fn new(borrow: &'a mut T, b2: &mut i32) -> Self where T: Copy {
      let de = *borrow;
      BorrowAndValue { borrow, value: de }
    }
}

#[derive(Copy, Clone)]
struct Foo(i32);

fn foo(x: &Foo) -> Foo {
  *x
}
