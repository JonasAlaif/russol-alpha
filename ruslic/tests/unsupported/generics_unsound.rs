use russol_contracts::*;

// Unsoundesses due to ignoring generics bounds

#[extern_spec]
fn leak<T>(b: Box<T>) -> &'static mut T {
    Box::leak(b)
}
// Ruslik will synth the following:
fn leak_synth<'a, T>(x: T) -> &'a mut T {
    let b = Box::new(x);
    leak(b)
}

// However, the rust compiler rejects this due to adding an implicit bound that
// `leak<T: 'static>`, but since we don't care about lifetime bounds on generics
// (and subtyping in general for that matter) we ignore these and synthesise
// a body which gets rejected. Note: this is purely due to the hacky way in which
// we handle generics.

// This is an identical issue:
#[extern_spec]
fn copy<T: Copy>(c: &T) -> T {
    *c
}
fn copy_synth<T>(c: &T) -> T {
    copy(c)
}
