#![no_std]

/// A macro for writing a precondition on a function.
pub use russol_macros::requires;
pub fn requires<R, T: Fn(R) -> bool>(_closure: T) {}

/// A macro for writing a postcondition on a function.
pub use russol_macros::ensures;
pub fn ensures<R, T: Fn(R) -> bool>(_closure: T) {}

/// A macro for writing a postcondition on a pure function.
pub use russol_macros::trusted_ensures;
pub fn trusted_ensures<R, T: Fn(R) -> bool>(_closure: T) {}

pub use russol_macros::extern_spec;
pub use russol_macros::helper;
pub use russol_macros::params;
/// A macro for writing a measure.
pub use russol_macros::pure;
pub use russol_macros::synth;

pub use russol_macros::ruslik;

/// This function is used to evaluate an expression in the “old”
/// context, that is at the beginning of the method call.
// pub fn old<T>(_arg: &T) -> T {
//     panic!("Cannot execute `old`, use `clone` instead.")
// }

/// A snapshot type
#[non_exhaustive]
#[derive(Copy, Clone)]
pub struct Snapshot<T: ?Sized>(core::marker::PhantomData<T>);
pub trait Snapshotable {
    fn snap(&self) -> Snapshot<Self> {
        panic!("Cannot take snapshot in executable code!")
    }
}
impl<T> Snapshotable for T {}
impl<T> Eq for Snapshot<T> {}
impl<T> PartialEq for Snapshot<T> {
    fn eq(&self, _: &Self) -> bool {
        panic!("Cannot compare snapshots in executable code!")
    }
}

// #[macro_export]
// macro_rules! equiv {
//     ($lhs:expr, $rhs:expr) => { ($lhs).snap() == ($rhs).snap() };
// }

/// A sequence type
#[non_exhaustive]
#[derive(Copy, Clone)]
pub struct Set<T>(core::marker::PhantomData<T>);
impl<T> Set<T> {
    pub fn new(_: &[&T]) -> Self {
        panic!()
    }
    // pub fn union(self, _: Self) -> Self { panic!() }
    // pub fn subset(self, _: Self) -> bool { panic!() }
    // pub fn diff(self, _: Self) -> Self { panic!() }
    // pub fn intersect(self, _: Self) -> Self { panic!() }
    // pub fn contains(self, _: i32) -> bool { panic!() }
}

#[macro_export]
macro_rules! set {
    ($($val:expr),*) => { $crate::Set::new(&[$($val,)*]) };
}
impl<T> core::ops::Index<&T> for Set<T> {
    type Output = bool;
    fn index(&self, _: &T) -> &bool {
        panic!()
    }
}
impl<T> core::ops::Add for Set<T> {
    type Output = Self;
    fn add(self, _: Self) -> Self {
        panic!()
    }
}
impl<T> core::ops::Sub for Set<T> {
    type Output = Self;
    fn sub(self, _: Self) -> Self {
        panic!()
    }
}
impl<T> core::ops::Mul for Set<T> {
    type Output = Self;
    fn mul(self, _: Self) -> Self {
        panic!()
    }
}
impl<T> Eq for Set<T> {}
impl<T> PartialEq for Set<T> {
    fn eq(&self, _: &Self) -> bool {
        panic!()
    }
}
impl<T> PartialOrd for Set<T> {
    fn partial_cmp(&self, _: &Self) -> Option<core::cmp::Ordering> {
        panic!()
    }
}
impl<T> Ord for Set<T> {
    fn cmp(&self, _other: &Self) -> core::cmp::Ordering {
        panic!()
    }
}
