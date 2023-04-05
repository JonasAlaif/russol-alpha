//! Example: test match expressions

use russol_contracts::*;

#[requires(x == 42)]
#[ensures(match result { 84 => true, 123 | 456 => false, _ => false })]
fn test_match_expr(x: u32) -> u32 {
  84 as u32
}

#[requires(x == -42)]
#[ensures(match result { Some(k) => k == -42, _ => false })]
fn test_match_option_expr(x: i32) -> Option<i32> {
  ::std::option::Option::Some((-42) as i32)
}
