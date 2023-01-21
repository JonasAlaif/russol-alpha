use russol_contracts::*;

// Using privacy to illustrate the point to avoid deconstructing and force copy
mod m {
    #[derive(Clone, Copy)]
    pub struct Priv(i32);
}

// Required to copy-out from behind ref
#[ensures(result === c)]
fn foo(c: &m::Priv) -> m::Priv {
    *c
}
