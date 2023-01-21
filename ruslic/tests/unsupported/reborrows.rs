use russol_contracts::*;

#[ensures(^*x == **x)]
fn foo<'b, 'a>(x: &'a mut &'b mut i32, y: &'b mut i32) -> &'a mut i32 {
    *x = y;
    // Cannot synthesize, because cannot write before open
    let tmp = &mut **x;
    tmp
    // expire tmp -> x
}

// It would like to do:
// let tmp = &mut **x;
// *x = y;
// tmp
// ^ But that is not valid code so it cannot
