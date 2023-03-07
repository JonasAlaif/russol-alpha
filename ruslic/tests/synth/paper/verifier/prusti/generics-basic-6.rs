use russol_contracts::*;

use std::marker::PhantomData;

struct Foo<A> {
    i: i32,
    x: BarBaz<A>,
}

struct BarBaz<B> {
    i: i32,
    x: PhantomData<B>,
}

#[ensures(result.i == arg.x.i)]
#[ensures(result.x.i == arg.i)]
fn test1<C, D>(arg: Foo<C>) -> Foo<D> {
  let x = BarBaz { i: arg.i as i32, x: ::std::marker::PhantomData };
  Foo { i: arg.x.i as i32, x }
}
