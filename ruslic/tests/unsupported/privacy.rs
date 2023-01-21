use russol_contracts::*;

mod m {
    // Private field
    pub struct Tuple(pub i32, i32);

    // Private struct
    #[derive(Clone, Copy)]
    struct Priv(i32);
    // Cannot touch second field
    pub struct HasPriv { pub x: i32, pub y: Priv }
    pub struct Twice(pub HasPriv, pub HasPriv);
}

// Cannot access private field - must mutate argument
#[ensures(result.0 == 42)]
fn foo(c: m::Tuple) -> m::Tuple {
    let mut c = c;
    c.0 = 42;
    c
}

// Cannot access private type
#[ensures(result.0.x == 4)]
#[ensures(result.1.x == 2)]
fn bar(c: m::HasPriv) -> m::Twice {
    // Not possible to do `y: c.y`
    let a = m::HasPriv { x: 4, ..c };
    let b = m::HasPriv { x: 2, ..c };
    m::Twice(a, b)
}
