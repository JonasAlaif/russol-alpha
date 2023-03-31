// https://stackoverflow.com/questions/29570781/temporarily-move-out-of-borrowed-content
use russol_contracts::*;

enum Foo<T> {
    Bar(T),
    Baz(T),
}

impl<T> Foo<T> {
    // FAILURE: (requires unsafe - std wrapped safe fns don't help!)
    // fn switch(&mut self) {
    //     todo!()
    // }
}
