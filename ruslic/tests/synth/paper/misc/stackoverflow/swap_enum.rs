// https://stackoverflow.com/questions/29570781/temporarily-move-out-of-borrowed-content
use russol_contracts::*;

enum Foo<T> {
    Bar(T),
    Baz(T),
}

impl<T> Foo<T> {
    // FAILURE: (requires unsafe - std wrapped safe fns don't help!)
    #[helper] // Skip synthesis
    fn switch(&mut self) {
        // *self = match self {
        //     &mut Foo::Bar(val) => Foo::Baz(val),
        //     &mut Foo::Baz(val) => Foo::Bar(val),
        // }
    }
}
