use russol_contracts::*;

#[ensures(result.0 === x.1 && result.1 === x.0)]
fn swap<T>(x: (T, T)) -> (T, T) {
  (x.1, x.0)
}
