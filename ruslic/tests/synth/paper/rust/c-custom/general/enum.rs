use russol_contracts::*;

enum Option {
    Left { left: i16 },
    Right { right: i16 },
}

#[requires(match *i {
    Option::Left { left } => left < i16::MAX,
    Option::Right { right } => right < i16::MAX
})]
#[ensures(match (i, result) {
    (Option::Left { left }, Option::Right { right }) => *left+1 == right,
    (Option::Right { right }, Option::Left { left }) => left == *right+1,
    _ => false,
})]
fn swap(i: &Option) -> Option {
  match i {
    Option::Left { left } => {
      let de = *left;
      Option::Right { right: (de + 1) as i16 }
    }
    Option::Right { right } => {
      let de = *right;
      Option::Left { left: (de + 1) as i16 }
    }
  }
}
