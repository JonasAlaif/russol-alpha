use russol_contracts::*;

// TODO: requires some simplification of `^left` in suslik after "AddToPost"
#[ensures(
    if cond {
        left === result && *right == ^right
    } else {
        right === result && *left == ^left
    }
)]
fn rbrrw<'a>(left: &'a mut i32, right: &'a mut i32, cond: bool) -> &'a mut i32 {
  if cond { left } else { right }
}
