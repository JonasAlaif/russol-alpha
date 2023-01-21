use russol_contracts::*;

#[requires(^x == 1)]
#[ensures(*x == 999)] // as hard as `false`
fn foo(x: &mut i32) {
  *x = 0 as i32;
  unreachable!()
}
