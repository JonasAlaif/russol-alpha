use russol_contracts::*;

#[ensures(if *ma >= *mb { *mb == ^mb && result === ma }
                    else { *ma == ^ma && result === mb })]
fn take_max<'a>(ma: &'a mut u16, mb: &'a mut u16) -> &'a mut u16 {
  let de_mb = *mb;
  let de_ma = *ma;
  if de_mb <= de_ma { ma } else { mb }
}
