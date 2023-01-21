use russol_contracts::*;

#[ensures(*result == **x)]
#[ensures(^result == *^x)]
#[ensures(^*x == ^^x)]
pub fn unnest<'a, 'b: 'a>(x: &'a mut &'b mut i32) -> &'a mut i32 {
  &mut **x
}
