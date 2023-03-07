use russol_contracts::*;

#[params("--solutions=1")]
#[requires(***x == 7_i32)]
#[ensures(  ^ ^^x == ^^y)]
#[ensures(  * ^^x == *^y)]
#[ensures(  ^ *^x == ^z)]
#[ensures(  **^x == 11_i32)]
// #[ensures(  ^^y == ^z)] // Impossible (with above) because after the call one could `**x = &mut (***x+1)` and suddenly `^^y != ^x`
fn foo<'a, 'b>(x: &mut &'b mut &'a mut i32, y: &'b mut &'a mut i32, z: &'a mut i32) {
  *z = 11 as i32;
  *y = z;
  *x = y
}

#[ensures(  ^^y == ^z)]
fn foo1<'a, 'b>(x: &mut &'b mut &'a mut i32, y: &'b mut &'a mut i32, z: &'a mut i32) {
  *y = z
}

#[ensures(*^x == ^result)]
// #[ensures(^*x == **x)] // Impossible
fn foo2<'a>(x: &'a mut &mut i32) -> &'a mut i32 {
  &mut **x
}

#[requires(**x + *y <= u16::MAX)]
#[ensures(*^x == **x + *y)]
#[ensures(^^x == ^y)]
fn foo3<'a>(x: &mut &'a mut u16, y: &'a mut u16) {
  let de = *y;
  let de_de = **x;
  *y = (de_de + de) as u16;
  *x = y
}

#[requires(**x <= 100_u8)]
#[requires(^result <= 100_u8)]
#[ensures(*^x <= 100_u8)]
#[ensures(*result <= 100_u8)]
fn foo4(x: &mut Box<u8>) -> &mut u8 {
  &mut **x
}
