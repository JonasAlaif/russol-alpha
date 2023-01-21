// https://stackoverflow.com/questions/52031002/how-do-i-move-out-of-a-struct-field-that-is-an-option
use russol_contracts::*;

struct SomeStruct<T> {
    attrib: T,
    next_attrib: Option<T>,
}

impl<T> SomeStruct<T> {
    #[ensures(if let Some(attr) = &self.next_attrib { (^self).attrib === attr } else { true })]
    // #[ensures(is_none(&(^self).next_attrib))]
    pub fn apply_changes(&mut self) {
      let result = replace(&mut self.next_attrib, ::std::option::Option::None);
      let new = match result {
        ::std::option::Option::None => ::std::option::Option::None,
        ::std::option::Option::Some(_0) => {
          let result = replace(&mut self.attrib, _0);
          ::std::option::Option::Some(result)
        }
      };
      self.next_attrib = new
    }
}

#[pure]
fn is_none<T>(opt: &Option<T>) -> bool { matches!(opt, None) }

#[extern_spec]
#[ensures(*dest === result)]
#[ensures(^dest === src)]
fn replace<T>(dest: &mut T, src: T) -> T { std::mem::replace(dest, src) }
