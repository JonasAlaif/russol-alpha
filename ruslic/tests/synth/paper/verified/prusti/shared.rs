use russol_contracts::*;

#[requires(*x == 5)]
#[ensures(*x == 5)]
pub fn test6(x: &u32) {
  ()
}

pub fn test<T>(x: &'static T) -> &'_ T {
  x
}
