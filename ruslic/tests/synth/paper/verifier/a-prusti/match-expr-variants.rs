//! Example: test match expressions

use russol_contracts::*;

#[requires(x == -42)]
#[ensures(match result { Some(..) => true, _ => false })]
fn test_match_expr(x: i32) -> Option<i32> {
  ::std::option::Option::Some(0 as i32)
}
