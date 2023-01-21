use russol_contracts::*;
struct Tuple<T> { a: T, b: T }
// Unsupported:
// #[ensures(result.0 === a && result.1 === b)]
// fn to_tuple<T>(Tuple { a, b }: Tuple<T>) -> (T, T) {
//     (a, b)
// }
#[ensures(result.0 === f.a && result.1 === f.b)]
fn to_tuple<T>(f: Tuple<T>) -> (T, T) {
  (f.a, f.b)
}
