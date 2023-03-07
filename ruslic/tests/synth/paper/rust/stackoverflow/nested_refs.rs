// https://stackoverflow.com/questions/32165917/why-does-linking-lifetimes-matter-only-with-mutable-references
use russol_contracts::*;

fn use_ref_ref<'a, 'b>(reference: &'a &'b mut ()) -> private::Token {
    private::use_same_ref_ref(reference)
}

mod private {
    use russol_contracts::*;
    pub struct Token(());

    #[extern_spec]
    pub fn use_same_ref_ref<'c>(reference: &'c &'c mut ()) -> Token {
        Token(())
    }
}
