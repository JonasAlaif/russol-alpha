use russol_contracts::*;

mod private {
    use russol_contracts::*;
    pub struct Percentage(u16);

    #[extern_spec]
    // TODO: this prevents the call atm:
    // #[requires(^result <= 100)]
    #[ensures(*result == p.0)]
    #[ensures(^result == (^p).0)]
    #[ensures(if ^result == 10 { is_ten(&^p) } else { true })]
    pub fn bar(p: &mut Percentage) -> &mut u16 {
        &mut p.0
    }

    #[pure]
    pub fn is_ten(p: &Percentage) -> bool {
        p.0 == 10
    }
}

#[ensures(private::is_ten(&^p))]
fn foo(p: &mut private::Percentage) {
    todo!()
}
