use russol_contracts::*;

#[ensures(^x == *y)]
#[ensures(^y == *x)]
fn swap(x: &mut i32, y: &mut i32) {
    let de_y = *y;
    let de_x = *x;
    *x = de_y as i32;
    *y = de_x as i32
}
