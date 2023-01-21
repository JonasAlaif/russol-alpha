use alloc::vec::Vec;
use proc_macro2::{Delimiter, Group, Punct, Spacing, Span, TokenStream, TokenTree};
use syn::{
    parse::Error, parse_quote, punctuated::Punctuated, spanned::Spanned, visit_mut::VisitMut,
    ReturnType,
};

use crate::SpecKind;

fn error_to_ts(e: Error, span: Span) -> TokenStream {
    e.to_compile_error()
        .into_iter()
        .map(|mut tt| {
            tt.set_span(span);
            tt
        })
        .collect::<TokenStream>()
}

pub(crate) fn parse_attr(
    mut attr: TokenStream,
    attr_kind: SpecKind,
    fn_ret_ty: &ReturnType,
) -> Result<syn::Stmt, TokenStream> {
    let span = attr.span();
    attr = TokenStreamRewrite::default().rewrite_stream(attr);
    let mut expr: syn::Expr = syn::parse2(attr).map_err(|e| error_to_ts(e, span))?;
    ExpressionRewrite::default().visit_expr_mut(&mut expr);
    let fn_name = match attr_kind {
        SpecKind::Requires => syn::Ident::new("requires", span),
        SpecKind::Ensures => syn::Ident::new("ensures", span),
        SpecKind::TrustedEnsures => syn::Ident::new("trusted_ensures", span),
    };
    Ok(match fn_ret_ty {
        ReturnType::Default => parse_quote! {
            #[allow(unused_parens)]
            ::russol_contracts::#fn_name(|_: ()| #expr);
        },
        ReturnType::Type(_, box ty) => parse_quote! {
            #[allow(unused_parens)]
            ::russol_contracts::#fn_name(|result: #ty| #expr);
        },
    })
}

#[derive(Default)]
struct TokenStreamRewrite {
    was_joint: bool,
    first_span: Option<Span>,
}
impl TokenStreamRewrite {
    fn mk_span_save(s: Span) -> TokenTree {
        let mut marker = TokenTree::Punct(Punct::new('*', Spacing::Alone));
        marker.set_span(s);
        marker
    }
    fn mk_marked(mut tt: TokenTree) -> TokenTree {
        let span = Span::call_site();
        assert!(ExpressionRewrite::is_fut_span(span));
        tt.set_span(span);
        tt
    }
    fn rewrite_stream(&mut self, ts: TokenStream) -> TokenStream {
        let peek = |i: &Option<TokenTree>| {
            if let TokenTree::Punct(p) = i.as_ref()? {
                Some(p.clone())
            } else {
                None
            }
        };
        let lookahead = Lookahead::double(ts.into_iter(), peek);
        lookahead
            .flat_map(|(tt, (la1, la2))| self.rewrite_tree(tt, la1, la2))
            .collect()
    }
    fn rewrite_tree(
        &mut self,
        tt: TokenTree,
        la1: Option<Punct>,
        la2: Option<Punct>,
    ) -> Option<TokenTree> {
        let was_joint = core::mem::replace(
            &mut self.was_joint,
            matches!(&tt, TokenTree::Punct(p) if p.spacing() == Spacing::Joint),
        );
        // I will use [OS] for tokens with original span and [MS] for tokens with a macro span;
        // the latter we can pick up in THIR (i.e. `*[OS]` -> normal deref vs `*[MS]` -> future deref)
        match tt {
            TokenTree::Group(g) => {
                let mut new_g = Group::new(g.delimiter(), self.rewrite_stream(g.stream()));
                new_g.set_span(g.span());
                Some(TokenTree::Group(new_g))
            }
            TokenTree::Punct(ref p) if p.as_char() == '^' => {
                // Prior to parsing this translates all `^[OS]` (only binary op) to `*[MS]*[OS]` (unary and binary op), otherwise e.g. `^x == 10` wouldn't parse
                // We translate both Future and XOr since we cannot distinguish them; `(^[OS]x) ^[OS] 1 == 1` -> `(*[MS]*[OS]x) *[MS] (*[OS]1) == 1`
                // However, this makes the parser succeed and we can correct our double translation later by walking the parsed expr.
                let outer = Self::mk_marked(TokenTree::Punct(Punct::new('*', p.spacing())));
                let inner = Self::mk_span_save(tt.span());
                Some(TokenTree::Group(Group::new(
                    Delimiter::None,
                    core::iter::once(outer)
                        .chain(core::iter::once(inner))
                        .collect(),
                )))
            }
            TokenTree::Punct(ref p) if p.as_char() == '=' => {
                match (self.first_span, was_joint, la1, la2) {
                    (_, false, Some(la1), Some(la2))
                        if p.spacing() == Spacing::Joint
                            && la1.as_char() == '='
                            && la1.spacing() == Spacing::Joint
                            && (la2.as_char() == '=' || la2.as_char() == '>') =>
                    {
                        assert_eq!(
                            la2.spacing(),
                            Spacing::Alone,
                            "The third symbol in `x =={} y` should have a space after it!",
                            la2.as_char()
                        );
                        self.first_span = Some(tt.span());
                        let new_char = if la2.as_char() == '>' {
                            '|'
                        } else {
                            la2.as_char()
                        };
                        Some(Self::mk_marked(TokenTree::Punct(Punct::new(
                            new_char,
                            Spacing::Joint,
                        ))))
                    }
                    (Some(_span), _, _, _) if p.spacing() == Spacing::Alone => {
                        self.first_span = None;
                        // Self::mk_span_save(tt.span().join(span).unwrap())
                        None
                    }
                    (Some(_), _, Some(la1), _) => {
                        assert!(p.spacing() == Spacing::Joint);
                        let new_char = if la1.as_char() == '>' {
                            '|'
                        } else {
                            la1.as_char()
                        };
                        Some(Self::mk_marked(TokenTree::Punct(Punct::new(
                            new_char,
                            Spacing::Alone,
                        ))))
                    }
                    _ => Some(tt),
                }
            }
            TokenTree::Punct(ref p) if p.as_char() == '>' => {
                if let Some(_span) = self.first_span && p.spacing() == Spacing::Alone {
                    self.first_span = None;
                    // Self::mk_span_save(tt.span().join(span).unwrap())
                    None
                } else { Some(tt) }
            }
            _ => Some(tt),
        }
    }
}

#[derive(Default)]
struct ExpressionRewrite {
    warn: bool,
    error: bool,
    hit_fut: bool,
}
impl ExpressionRewrite {
    fn is_fut_span(span: Span) -> bool {
        span.start() == Span::call_site().start() && span.end() == Span::call_site().end()
    }
    fn clean_stream(&mut self, ts: TokenStream) -> TokenStream {
        ts.into_iter()
            .filter_map(|tt| self.single_deref(tt))
            .collect()
    }
    fn single_deref(&mut self, tt: TokenTree) -> Option<TokenTree> {
        let has_fut_span = Self::is_fut_span(tt.span());
        match tt {
            TokenTree::Group(g) => {
                assert!(!self.hit_fut);
                let mut new_g = Group::new(g.delimiter(), self.clean_stream(g.stream()));
                new_g.set_span(g.span());
                Some(TokenTree::Group(new_g))
            }
            TokenTree::Punct(ref p) if p.as_char() == '*' => {
                if self.hit_fut {
                    assert!(!has_fut_span);
                    self.hit_fut = false;
                    if !self.warn {
                        self.warn = true;
                    }
                    None
                } else {
                    self.hit_fut = has_fut_span;
                    Some(tt)
                }
            }
            TokenTree::Punct(ref p)
                if (p.as_char() == '|' || p.as_char() == '=') && has_fut_span =>
            {
                if !self.error {
                    self.error = true;
                }
                if self.hit_fut {
                    self.hit_fut = false;
                    let first = TokenTree::Punct(Punct::new('=', Spacing::Joint));
                    let second = TokenTree::Punct(Punct::new(
                        if p.as_char() == '|' { '>' } else { p.as_char() },
                        Spacing::Alone,
                    ));
                    Some(TokenTree::Group(Group::new(
                        Delimiter::None,
                        core::iter::once(first)
                            .chain(core::iter::once(second))
                            .collect(),
                    )))
                } else {
                    self.hit_fut = true;
                    Some(TokenTree::Punct(Punct::new('=', Spacing::Joint)))
                }
            }
            _ => {
                assert!(!self.hit_fut);
                self.hit_fut = has_fut_span;
                Some(tt)
            }
        }
    }
    fn wrap_in_paren(expr: syn::Expr) -> syn::Expr {
        syn::Expr::Paren(syn::ExprParen {
            attrs: Vec::new(),
            paren_token: syn::token::Paren { span: expr.span() },
            expr: box expr,
        })
    }
    fn call_snapshot(expr: syn::Expr) -> syn::ExprMethodCall {
        let span = expr.span();
        syn::ExprMethodCall {
            attrs: Vec::new(),
            receiver: box Self::wrap_in_paren(expr),
            dot_token: syn::Token![.](span),
            method: syn::Ident::new("snap", span),
            turbofish: None,
            paren_token: syn::token::Paren { span },
            args: Punctuated::new(),
        }
    }
}
impl syn::visit_mut::VisitMut for ExpressionRewrite {
    fn visit_expr_mut(&mut self, node: &mut syn::Expr) {
        syn::visit_mut::visit_expr_mut(self, node);
        if let syn::Expr::Unary(expr) = node &&
            matches!(expr.op, syn::UnOp::Deref(..)) && Self::is_fut_span(expr.op.span())
        {
            // For the expression `*[OS]^[OS]x` if we just naively translated to `*[OS]*[MS]x` and gave it to rustc
            // there seems to be a bug that makes such an expr appear in THIR as `*[MS]*[MS]x`! To workaround this we
            // translate `*[OS]*[MS]*[OS]x` (from preparser of `*[OS]^[OS]x`) to `*[OS]([OS]*[MS]x)`, since the `([OS]...)`
            // with the original span seems to prevent the `*[OS]...` from being corrupted.
            if let syn::Expr::Unary(mut expr) = core::mem::replace(node, syn::Expr::Verbatim(TokenStream::new())) {
                if let box syn::Expr::Unary(inner) = &mut expr.expr {
                    core::mem::swap(&mut inner.op, &mut expr.op);
                    core::mem::swap(&mut inner.attrs, &mut expr.attrs);
                } else { unreachable!() }
                let paren_token = syn::token::Paren { span: expr.op.span() };
                *node = syn::Expr::Paren(syn::ExprParen { attrs: Vec::new(), paren_token, expr: expr.expr });
            } else { unreachable!() }
        } else if let syn::Expr::Binary(expr) = node &&
            matches!(expr.op, syn::BinOp::Mul(..)) && Self::is_fut_span(expr.op.span())
        {
            // Recognise patterns which should've been XOr and revert them: `x *[MS] (*[OS]1) == 1` -> `x ^[OS] 1 == 1`
            let right = core::mem::replace(&mut *expr.right, syn::Expr::Verbatim(TokenStream::new()));
            fn deconstruct_right(expr: &mut syn::ExprBinary, right: syn::Expr) -> Option<syn::Expr> {
                if let syn::Expr::Unary(right) = right {
                    if !matches!(right.op, syn::UnOp::Deref(_)) { return Some(syn::Expr::Unary(right)); }
                    match (&right.expr, &expr.left) {
                        (box syn::Expr::Paren(_), box syn::Expr::Paren(_)) => {
                            expr.op = syn::BinOp::BitXor(syn::Token![^](right.op.span()));
                            expr.right = right.expr;
                            None
                        }
                        _ => Some(*right.expr),
                    }
                } else { Some(right) }
            }
            let error = deconstruct_right(expr, right);
            use quote::ToTokens;
            if let Some(right) = error {
                proc_macro::Diagnostic::spanned(
                    expr.span().unwrap(),
                    proc_macro::Level::Error,
                    alloc::format!(
                        "Bitwise XOr `x ^ y` only supported if surrounding arguments ({} ^ {}) are in brackets: `(x) ^ (y)`.",
                        expr.left.to_token_stream(),
                        right.to_token_stream()
                    )
                ).emit();
            } else {
                proc_macro::Diagnostic::spanned(
                    expr.span().unwrap(),
                    proc_macro::Level::Warning,
                    alloc::format!(
                        "Bitwise XOr is poorly supported even with brackets, e.g. `(x) ^ (y) & z` should be parsed as `x ^ (y & z)` but will instead be parsed as `(x ^ y) & z. I parsed: {}",
                        expr.to_token_stream(),
                    )
                ).emit();
            }
        } else if let syn::Expr::Binary(expr) = node &&
            matches!(expr.op, syn::BinOp::Eq(..)) && Self::is_fut_span(expr.op.span())
        {
            let left = core::mem::replace(&mut *expr.left, syn::Expr::Verbatim(TokenStream::new()));
            let right = core::mem::replace(&mut *expr.right, syn::Expr::Verbatim(TokenStream::new()));
            let span: Span = left.span().unwrap().after().join(right.span().unwrap().before()).unwrap().into();
            expr.op = syn::BinOp::Eq(syn::Token![==](span));
            expr.left = box syn::Expr::MethodCall(Self::call_snapshot(left));
            expr.right = box syn::Expr::MethodCall(Self::call_snapshot(right));
        } else if let syn::Expr::Binary(expr) = node &&
            matches!(expr.op, syn::BinOp::Or(..)) && Self::is_fut_span(expr.op.span())
        {
            let left = core::mem::replace(&mut *expr.left, syn::Expr::Verbatim(TokenStream::new()));
            let span: Span = left.span().unwrap().after().join(expr.right.span().unwrap().before()).unwrap().into();
            expr.op = syn::BinOp::Or(syn::Token![||](span));
            expr.left = box syn::Expr::Unary(syn::ExprUnary {
                attrs: Vec::new(),
                op: syn::UnOp::Not(syn::Token![!](left.span())),
                expr: box Self::wrap_in_paren(left),
            });
            // Beware that `x ==> y || z` will parse as `(!(x) || y) || z` even though we might want `!(x) || (y || z)`!
        } else if let syn::Expr::Macro(node) = node {
            let tokens = core::mem::take(&mut node.mac.tokens);
            node.mac.tokens = tokens.into_iter().filter_map(|tt| self.single_deref(tt)).collect();
            use quote::ToTokens;
            if self.error {
                proc_macro::Diagnostic::spanned(
                    node.span().unwrap(),
                    proc_macro::Level::Error,
                    alloc::format!(
                        "Equivalence `x === y` and implication `x ==> y` are not supported inside macro calls: `{}!({})`. Use `(x).snap() == (y).snap()` or `!(x) || y` instead.",
                        node.mac.path.to_token_stream(),
                        node.mac.tokens
                    )
                ).emit();
                self.error = false;
            } else if self.warn {
                proc_macro::Diagnostic::spanned(
                    node.span().unwrap(),
                    proc_macro::Level::Warning,
                    alloc::format!(
                        "Encountered a caret '^' inside a macro, it has been turned into a '*': `{}!({})`. This leads to XOr turning into Mult. Also beware that `*^x` will be read as `^^x`, use `*(^x)` where needed.",
                        node.mac.path.to_token_stream(),
                        node.mac.tokens
                    )
                ).emit();
                self.warn = false;
            }
        }
    }
}

pub struct Lookahead<
    I: Iterator,
    P,
    F: Fn(&[Option<I::Item>], &[Option<I::Item>]) -> P,
    const N: usize,
> {
    iter: I,
    popped: [Option<I::Item>; N],
    idx: usize,
    f: F,
}
// impl<I: Iterator, P, F: Fn(&[Option<I::Item>], &[Option<I::Item>]) -> P, const N: usize> Lookahead<I, P, F, N> {
//     fn new(mut iter: I, f: F) -> Lookahead<I, P, F, N> {
//         let popped: [Option<_>; N] = [(); N].map(|_| iter.next());
//         Lookahead { iter, popped, idx: 0, f }
//     }
// }
// impl<I: Iterator, P> Lookahead<I, P, alloc::boxed::Box<dyn Fn(&[Option<I::Item>], &[Option<I::Item>]) -> P>, 1> {
//     fn single<F1: Fn(&Option<I::Item>) -> P + 'static>(mut iter: I, f: F1) -> Lookahead<I, P, alloc::boxed::Box<dyn Fn(&[Option<I::Item>], &[Option<I::Item>]) -> P>, 1> {
//         let popped: [Option<_>; 1] = [iter.next()];
//         Lookahead { iter, popped, idx: 0, f: alloc::boxed::Box::new(move |n, _| (f)(&n[0])) }
//     }
// }
type LaHd<I, P> = Lookahead<
    I,
    (P, P),
    alloc::boxed::Box<
        dyn Fn(&[Option<<I as Iterator>::Item>], &[Option<<I as Iterator>::Item>]) -> (P, P),
    >,
    2,
>;
impl<I: Iterator, P> LaHd<I, P> {
    fn double<F1: Fn(&Option<I::Item>) -> P + 'static>(mut iter: I, f: F1) -> LaHd<I, P> {
        let popped: [Option<_>; 2] = [iter.next(), iter.next()];
        Lookahead {
            iter,
            popped,
            idx: 0,
            f: alloc::boxed::Box::new(move |e, s| {
                (
                    (f)(&e[0]),
                    if e.len() == 2 { (f)(&e[1]) } else { (f)(&s[0]) },
                )
            }),
        }
    }
}
impl<I: Iterator, P, F: Fn(&[Option<I::Item>], &[Option<I::Item>]) -> P, const N: usize> Iterator
    for Lookahead<I, P, F, N>
{
    type Item = (I::Item, P);

    fn next(&mut self) -> Option<Self::Item> {
        let future = self.iter.next();
        let next = core::mem::replace(&mut self.popped[self.idx], future);
        next.map(|n| {
            self.idx = (self.idx + 1) % N;
            (
                n,
                (self.f)(&self.popped[self.idx..], &self.popped[..self.idx]),
            )
        })
    }
}
