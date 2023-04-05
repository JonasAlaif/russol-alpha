//! Example: test specification of box dereferentiation

use russol_contracts::*;

#[ensures(result == *my_box)]
fn foo(my_box: Box<i32>) -> i32 {
  *my_box
}
