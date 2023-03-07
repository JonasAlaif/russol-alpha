use russol_contracts::*;

#[extern_spec]
#[ensures(result == if a + b >= 256 { a + b - 256 } else { a + b })]
pub fn wrapping_add(a: u8, b: u8) -> u8 {
    a.wrapping_add(b)
}

#[ensures(result == a + b || result == a + b - 256)]
pub fn test_u8_wrapping_add(a: u8, b: u8) -> u8 {
  let result = wrapping_add(b as u8, a);
  if 256 <= b + a { result } else { result }
}
