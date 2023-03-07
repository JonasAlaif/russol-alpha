use russol_contracts::*;

struct Number<A, B, C> {
    a: A,
    b: B,
    c: C,
}

#[requires(-10_000 < arg.b && arg.b < 10_000)]
#[ensures((^arg).b == arg.b - 1000)]
fn test1<A, B>(arg: &mut Number<A, i32, B>) {
  let de = arg.b;
  arg.b = (de - 1000) as i32
}

#[requires(-10_000 < arg.b.b && arg.b.b < 10_000)]
#[ensures((^arg).b.b == arg.b.b - 1000)]
fn test2<A, B, C, D>(arg: &mut Number<A, Number<B, i32, C>, D>) {
  let de = arg.b.b;
  arg.b.b = (de - 1000) as i32
}
