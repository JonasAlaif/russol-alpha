use rustc_ast::{AttrItem, AttrKind, MacArgs, MacArgsEq, NormalAttr};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    def::DefKind,
    def_id::{DefId, LocalDefId},
};
use rustc_middle::{
    thir::{ClosureExpr, Expr, ExprId, ExprKind, Stmt, StmtKind, Thir},
    ty::{DefIdTree, TyCtxt, TyKind, WithOptConstParam},
};

use crate::ruslik_ssl::Var;
use crate::{contract_translator::to_expr, ruslik_pure::PureExpression, ruslik_types::RuslikFnSig};

#[derive(Debug, Clone)]
pub struct PureFn<'tcx> {
    pub def_id: DefId,
    pub arg_names: Vec<Var>,
    pub expr: PureExpression<'tcx>,
    pub pure_post: PureExpression<'tcx>,
    pub executable: bool,
    pub ast_nodes: usize,
}

pub type PureFnMap<'tcx> = FxHashMap<DefId, PureFn<'tcx>>;

#[derive(Clone, Copy, Eq, PartialEq)]
enum SpecKind {
    Requires,
    Ensures,
    TrustedEnsures,
}

pub struct HirTranslator<'tcx> {
    tcx: TyCtxt<'tcx>,
    pub pure_fns: PureFnMap<'tcx>,
    pub extern_fns: Vec<RuslikFnSig<'tcx>>,
    pub impure_fns: Vec<(bool, RuslikFnSig<'tcx>)>,
    // Type predicates
    // types: FxHashMap<DefId, RusType<'tcx>>,
    // basic_types: FxHashSet<RusType<'tcx>>,
}
impl<'tcx> HirTranslator<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            pure_fns: FxHashMap::default(),
            extern_fns: Vec::new(),
            impure_fns: Vec::new(),
            // types: FxHashMap::default(),
            // basic_types: FxHashSet::default(),
        }
    }

    pub fn translate(&mut self, def_id: DefId) -> Option<()> {
        match self.tcx.def_kind(def_id) {
            DefKind::AssocFn | DefKind::Fn => {
                // Don't try to synth derived fns
                let span: rustc_span::Span = self.tcx.def_span(def_id);
                if span.from_expansion() {
                    return None;
                }
                // Don't try to synth default trait fns
                let parent = self
                    .tcx
                    .opt_parent(def_id)
                    .map(|parent| self.tcx.def_kind(parent));
                if matches!(
                    parent,
                    Some(DefKind::Trait) | Some(DefKind::Fn) | Some(DefKind::AssocFn)
                ) {
                    return None;
                }

                let (mut is_pure, mut is_extern, mut is_synth, mut params) =
                    (false, false, false, String::new());
                for a in self.tcx.get_attrs_unchecked(def_id) {
                    if let AttrKind::Normal(p) = &a.kind {
                        let NormalAttr {
                            item: AttrItem { path, args, .. },
                            ..
                        } = &**p;
                        match path.segments.last().unwrap().ident.as_str() {
                            "ruslik_helper" => return None,
                            "ruslik_pure" => is_pure = true,
                            "ruslik_extern_spec" => is_extern = true,
                            "ruslik_synth" => is_synth = true,
                            "ruslik_params" => {
                                if let MacArgs::Eq(_, MacArgsEq::Hir(lit)) = args {
                                    params = params + " " + lit.token_lit.symbol.as_str();
                                } else {
                                    unreachable!()
                                }
                            }
                            _ => (),
                        }
                    }
                }
                let (pure_pre, pure_post, ast_nodes) = self.collect_contracts(def_id, is_pure)?;
                if is_pure {
                    assert!(
                        !is_extern,
                        "Cannot have `#[pure]` and `#[extern_spec]` on the same function!"
                    );
                    assert!(
                        params.is_empty(),
                        "Cannot have params on `#[pure]`: \"{}\"",
                        params
                    );
                    let (expr, pure_nodes) = to_expr(self.tcx, def_id.expect_local());
                    let arg_names = self
                        .tcx
                        .fn_arg_names(def_id)
                        .iter()
                        .map(|id| Var::arg(id.name))
                        .collect();
                    // Get return value
                    let gen_sig = self.tcx.fn_sig(def_id);
                    let sig: rustc_middle::ty::FnSig =
                        self.tcx.liberate_late_bound_regions(def_id, gen_sig);
                    let executable = sig.inputs().len() == 1
                        && !sig.inputs()[0]
                            .to_string()
                            .starts_with("russol_contracts::")
                        && !sig.output().to_string().starts_with("russol_contracts::");
                    let pure_fn = PureFn {
                        def_id,
                        arg_names,
                        expr,
                        pure_post,
                        executable,
                        ast_nodes: ast_nodes + pure_nodes,
                    };
                    self.pure_fns.insert(def_id, pure_fn);
                } else {
                    if is_extern {
                        assert!(
                            params.is_empty(),
                            "Cannot have params on `#[extern_spec]`: \"{}\"",
                            params
                        );
                    }
                    self.tcx.ensure().mir_borrowck(def_id.expect_local());
                    let sig =
                        RuslikFnSig::new(def_id, self.tcx, pure_pre, pure_post, params, ast_nodes);
                    if is_extern {
                        self.extern_fns.push(sig);
                    } else {
                        self.impure_fns.push((is_synth, sig));
                    }
                }
            }
            DefKind::Closure
            | DefKind::AnonConst
            | DefKind::Const
            | DefKind::Static(_)
            | DefKind::AssocConst => (),
            other => println!("Skipping {:?}", other),
        }
        Some(())
    }

    fn parse_attr_count(&self, def_id: DefId) -> Option<usize> {
        self.tcx.get_attrs_unchecked(def_id).iter().find_map(|a| {
            if let AttrKind::Normal(p) = &a.kind {
                if let NormalAttr {
                    item:
                        AttrItem {
                            path,
                            args: MacArgs::Eq(_, MacArgsEq::Hir(l)),
                            ..
                        },
                    ..
                } = &**p
                {
                    if path.segments.last().unwrap().ident.to_string() == "ruslik_spec_count" {
                        Some(l.token_lit.symbol.as_str().parse::<usize>().ok()?)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    fn collect_contracts(
        &self,
        def_id: DefId,
        is_pure: bool,
    ) -> Option<(PureExpression<'tcx>, PureExpression<'tcx>, usize)> {
        let (thir, _) = self
            .tcx
            .thir_body(WithOptConstParam::unknown(def_id.expect_local()))
            .unwrap();
        if format!("{thir:?}") == "Steal { value: RwLock(RefCell { value: None }) }" {
            // let span: rustc_span::Span = self.tcx.def_span(def_id);
            // (span, "stolen body!")
            return None;
        }
        let thir = &thir.borrow();
        let mut contracts_len = 0;
        let contract = thir
            .stmts
            .iter()
            .map_while(|stmt| self.parse_spec_stmt(stmt, thir))
            .fold(
                (
                    PureExpression::from_bool(true, self.tcx),
                    PureExpression::from_bool(true, self.tcx),
                    0,
                ),
                |acc, (kind, cid)| {
                    assert_eq!(is_pure, kind == SpecKind::TrustedEnsures);
                    let (mut acc_pre, mut acc_post, ast_nodes) = acc;
                    let (expr, new_nodes) = to_expr(self.tcx, cid);
                    contracts_len += 1;
                    if kind == SpecKind::Requires {
                        acc_pre = acc_pre & expr;
                    } else {
                        acc_post = acc_post & expr;
                    }
                    (acc_pre, acc_post, ast_nodes + new_nodes)
                },
            );

        if let Some(attrs) = self.parse_attr_count(def_id) {
            assert_eq!(
                contracts_len, attrs,
                "Unexpected THIR layout! Could not find all specs."
            );
        } else {
            assert_eq!(
                contracts_len, 0,
                "Could not find ruslik_spec_count attribute, even though specs are present!"
            );
            // if !is_pure { println!("Found no contract for {}. Unconstrained synthesis is generally uninteresting!", self.tcx.item_name(def_id)); }
        }
        Some(contract)
    }
    fn parse_spec_stmt(&self, stmt: &Stmt, thir: &Thir) -> Option<(SpecKind, LocalDefId)> {
        if let StmtKind::Expr { expr, .. } = stmt.kind {
            if let ExprKind::Call {
                fun,
                args: box [arg],
                ..
            } = Self::descope_expr(expr, thir)?.kind
            {
                let fun: &Expr = Self::descope_expr(fun, thir)?;
                if let TyKind::FnDef(def_id, _) = fun.ty.kind()
                    && let ExprKind::Closure(box ClosureExpr { closure_id, .. }) = Self::descope_expr(arg, thir)?.kind
                    && self.tcx.crate_name(def_id.krate).to_string() == "russol_contracts"
                {
                    match self.tcx.item_name(*def_id).to_string().as_str() {
                        "requires" => Some((SpecKind::Requires, closure_id)),
                        "ensures" => Some((SpecKind::Ensures, closure_id)),
                        "trusted_ensures" => Some((SpecKind::TrustedEnsures, closure_id)),
                        _ => None,
                    }
                    // println!("crate_name: {}", self.tcx.crate_name(def_id.krate));
                    // println!("item_name: {}", self.tcx.item_name(*def_id));
                    // println!("fun: {:#?}", fun.ty.kind());
                    // println!("arg: {:#?}", Self::descope_expr(arg, &thir));
                } else { None }
            } else {
                None
            }
        } else {
            None
        }
    }
    /// Ok to return None (rather than panic), since we later check that we have parsed the correct amount of specs
    fn descope_expr<'thir, 'b>(expr: ExprId, thir: &'b Thir<'thir>) -> Option<&'b Expr<'thir>> {
        if let ExprKind::Scope { value, .. } = thir.exprs[expr].kind {
            Some(&thir.exprs[value])
        } else {
            None
        }
    }
}
