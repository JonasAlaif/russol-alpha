#![allow(dead_code)]

use russol_contracts::*;

struct T {
    val: i32
}

#[ensures((^x).val == (^result).val)]
fn identity(x: &mut T) -> &mut T {
  x
}

#[ensures(result.val == v)]    // TODO x.val is illegal, but Prusti doesn't report a readable error message.
#[ensures((^x).val == (^result).val)]
fn identity2(x: &mut T, v: i32) -> &mut T {
  let new = T { val: v };
  *x = new;
  x
}

#[ensures(*result == v)]
#[ensures((^x).val == (^result))]
fn identity3(x: &mut T, v: i32) -> &mut i32 {
  x.val = v as i32;
  &mut x.val
}
