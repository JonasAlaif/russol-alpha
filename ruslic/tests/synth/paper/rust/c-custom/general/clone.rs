fn clone<T: Clone>(x: &T) -> T {
  x.clone()
}
struct Foo<T>(T);
impl<T: Clone> Foo<T> {
    fn clone(&self) -> Self {
      let result = self.0.clone();
      Foo(result)
    }
}
