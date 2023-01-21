#![feature(box_patterns)]

use russol_contracts::*;

// struct FullA {
//     a: i32,
//     b: Common,
//     c: Common,
// }

struct End { x: i32, y: Common, }
struct Common { f3: u8, f4: i32 }
struct Start { z: Enum }
enum Enum {
    V1 { f5: Common },
    V2 { f6: i32, f7: Common },
}

// #[requires(matches!(&x.z, Enum::V2 { .. }))]

// #[ensures(match x.z {
//     Enum::V1 { .. } => false,
//     Enum::V2 { f6, .. } => result.x == f6 - i,
// })]
// fn rearrange(x: Start, i: i32) -> End {
//     ruslik!()
// }


#[params("--solutions=1")]
#[requires(matches!(x.z, Enum::V2 { f6, .. } if f6 >= i.f4 && i.f4 >= 0))]
#[ensures(match (&x).z {
    Enum::V1 { f5: ref f5@Common { f3, .. } } => f5.f3 == f3,
    // Enum::V1 { .. } => false,
    Enum::V2 { f6, .. } => result.x == f6 - i.f4,
})]
// #[ensures(^i == 10)]

// #[requires(i > 10 && i < 10)]
// #[ensures(result.x + 2 >= x.a + (-1))]
fn swap(i: &mut Common, x: Start) -> End {
  let de = i.f4;
  match x.z {
    Enum::V1 { .. } => unreachable!(),
    Enum::V2 { f6, f7 } => End { x: (f6 - de) as i32, y: f7 },
  }
}
