#![allow(dead_code, non_snake_case)]
use russol_contracts::*;

struct Account {
    bal: u16,
}

impl Account {

    #[pure]
    fn balance(&self) -> u16 {
        self.bal
    }

    #[requires(self.balance() + amount <= u16::MAX)]
    #[ensures((^self).balance() == self.balance() + amount)]
    fn deposit(&mut self, amount: u16) {
      let de = self.bal;
      let new = Account { bal: (de + amount) as u16 };
      *self = new
    }

    #[requires(amount <= self.balance())]
    #[ensures((^self).balance() == self.balance() - amount)]
    fn withdraw(&mut self, amount: u16) {
      let de = self.bal;
      let new = Account { bal: (de - amount) as u16 };
      *self = new
    }

    #[requires(other.balance() + amount <= u16::MAX)]
    #[requires(amount <= self.balance())]
    #[ensures((^self).balance() == self.balance() - amount)]
    #[ensures((^other).balance() == other.balance() + amount)]
    fn transfer(&mut self, other: &mut Account, amount: u16) {
      let de_bal_other = other.bal;
      let de_bal_self = self.bal;
      self.bal = (de_bal_self - amount) as u16;
      other.bal = (de_bal_other + amount) as u16
    }
}
