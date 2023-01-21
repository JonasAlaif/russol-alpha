use russol_contracts::*;

#[pure]
fn is_some<T>(x: &Option<T>) -> bool {
    matches!(x, Some(_))
}
#[pure]
fn is_ok<T, E>(x: &Result<T, E>) -> bool {
    matches!(x, Ok(_))
}

struct Bar<T> { data: Option<Box<T>> }
impl<T> Bar<T> {
    // User code: (https://stackoverflow.com/questions/22282117/how-do-i-borrow-a-reference-to-what-is-inside-an-optiont)
    #[ensures(is_some(&self.data) == is_ok(&result))]
    fn borrow(&mut self) -> Result<&Box<T>, ()> {
      match &mut self.data {
        ::std::option::Option::None => ::std::result::Result::Err(()),
        ::std::option::Option::Some(_0) => ::std::result::Result::Ok(_0),
      }
    }
}
