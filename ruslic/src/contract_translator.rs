use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    def_id::{DefId, LocalDefId},
    HirId,
};
use rustc_middle::{
    mir::Field,
    thir::{
        self, AdtExpr, ArmId, BindingMode, Block, ExprId, ExprKind, Guard, Pat, PatKind, StmtId,
        StmtKind, Thir,
    },
    ty::{adjustment::PointerCast, SubstsRef, TyCtxt, TyKind, WithOptConstParam},
};
use rustc_target::abi::VariantIdx;

use crate::{
    constant::{translate_constant, try_to_bits},
    ruslik_pure::{self, BuiltinCallKind, CallInfo, ExprKind as Expr, PureExpression, UnOpKind},
    ruslik_ssl::Var,
    ruslik_types::AdtIdent,
};

pub type VarMap<'tcx> = FxHashMap<rustc_hir::HirId, ruslik_pure::PureExpression<'tcx>>;

pub fn to_expr<'tcx>(tcx: TyCtxt<'tcx>, id: LocalDefId) -> (PureExpression<'tcx>, usize) {
    // println!("Translating {id:?} to expr.");
    let (thir, expr) = tcx.thir_body(WithOptConstParam::unknown(id)).unwrap();
    let thir = thir.borrow();
    // if thir.exprs.is_empty() {
    //     return Err(Error::new(tcx.def_span(id), "type checking failed"));
    // };
    let mut thir_term = ThirTerm {
        tcx,
        item_id: id,
        thir: &thir,
        var_map: FxHashMap::default(),
        ast_nodes: 0,
    };
    (thir_term.expr_term(expr), thir_term.ast_nodes)
}

struct ThirTerm<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    item_id: LocalDefId,
    thir: &'a Thir<'tcx>,
    var_map: VarMap<'tcx>,
    ast_nodes: usize,
}

impl<'a, 'tcx> ThirTerm<'a, 'tcx> {
    fn expr_term(&mut self, expr: ExprId) -> PureExpression<'tcx> {
        // let ty = self.thir[expr].ty;
        // eprintln!("{:?}", &thir[expr].kind);
        let ty = self.thir[expr].ty;
        // println!("Looking at expr: {:?} of ty: {}", self.thir[expr].kind, ty);
        let expr = &self.thir[expr];
        match expr.kind {
            ExprKind::Scope { value, .. } => self.expr_term(value),
            ExprKind::Block { block } => {
                let Block { ref stmts, expr, span, .. } = self.thir[block];
                for stmt in stmts.iter() {
                    self.stmt_term(*stmt);
                }
                let inner = match expr {
                    Some(e) => self.expr_term(e),
                    None => panic!("block with no terminator {span:?}"),
                };

                inner
            }
            ExprKind::Binary { op, lhs, rhs } => {
                self.ast_nodes += 1;
                Expr::BinOp(op.to_hir_binop(), box self.expr_term(lhs), box self.expr_term(rhs)).with_ty(ty)
            }
            ExprKind::LogicalOp { op, lhs, rhs } => {
                self.ast_nodes += 1;
                let op = match op {
                    thir::LogicalOp::And => rustc_hir::BinOpKind::And,
                    thir::LogicalOp::Or => rustc_hir::BinOpKind::Or,
                };
                Expr::BinOp(op, box self.expr_term(lhs), box self.expr_term(rhs)).with_ty(ty)
            }
            ExprKind::Unary { op, arg } => {
                self.ast_nodes += 1;
                Expr::UnOp(UnOpKind::UnOp(op), box self.expr_term(arg)).with_ty(ty)
            }
            // Result or local var:
            ExprKind::VarRef { id } => {
                self.ast_nodes += 1;
                if let Some(e) = self.var_map.get(&id.0) {
                    e.clone()
                } else {
                    let name = self.tcx.hir().name(id.0);
                    // Can also be arg of a pure function
                    // assert!(name.as_str() == "result", "Non-result VarRef encountered: {}", name);
                    Expr::Var(Var::arg(name)).with_ty(ty)
                }
            }
            // FnArg:
            ExprKind::UpvarRef { var_hir_id: id, .. } => {
                self.ast_nodes += 1;
                assert!(!self.var_map.contains_key(&id.0));
                Expr::Var(Var::arg(self.tcx.hir().name(id.0))).with_ty(ty)
            }
            ExprKind::Literal { lit, neg } => {
                self.ast_nodes += 1;
                let lit = Expr::Lit(lit.node.clone()).with_ty(ty);
                if neg { -lit } else { lit }
            }
            ExprKind::Call { ty: fn_ty, ref args, .. } if let TyKind::FnDef(id, substs) = fn_ty.kind() => {
                self.ast_nodes += 1;
                let mut arg_exprs: Vec<_> = args.iter().map(|arg| self.expr_term(*arg)).collect();
                let ci = match self.get_stub_kind(*id, substs) {
                    None => CallInfo::Pure(*id, substs),
                    Some(RuslikStub::Snap) => {
                        self.ast_nodes -= 1; // Automatically added by `===`
                        assert!(arg_exprs.len() == 1); assert!(ty.to_string().starts_with("russol_contracts::Snapshot"));
                        let arg = arg_exprs.pop().unwrap().deref(false);
                        let ty = arg.ty();
                        return Expr::UnOp(UnOpKind::Snap, box arg).with_ty(ty);
                    }
                    Some(RuslikStub::SetNew) => {
                        assert!(args.len() == 1); assert!(ty.to_string().starts_with("russol_contracts::Set"));
                        let deref = arg_exprs.pop().unwrap().deref(false);
                        arg_exprs.push(deref);
                        CallInfo::Builtin(BuiltinCallKind::SetConstruct)
                    }
                    Some(RuslikStub::In) => {
                        assert!(args.len() == 2);
                        let elem = arg_exprs.pop().unwrap().deref(false);
                        let set = arg_exprs.pop().unwrap().deref(false);
                        assert!(set.ty().to_string().starts_with("russol_contracts::Set"), "{set:?}");
                        arg_exprs.push(set);
                        arg_exprs.push(elem);
                        CallInfo::Builtin(BuiltinCallKind::SetContains)
                    }
                    Some(s@RuslikStub::Add) |
                    Some(s@RuslikStub::Sub) |
                    Some(s@RuslikStub::Eq) |
                    Some(s@RuslikStub::Ge) |
                    Some(s@RuslikStub::Le) |
                    Some(s@RuslikStub::Gt) |
                    Some(s@RuslikStub::Lt) => {
                        assert_eq!(args.len(), 2);
                        let rhs = arg_exprs.pop().unwrap();
                        let lhs = arg_exprs.pop().unwrap();
                        assert_eq!(lhs.ty(), rhs.ty());
                        return Expr::BinOp(
                            s.to_binop(),
                            box if s.expect_deref() { lhs.deref(false) } else { lhs },
                            box if s.expect_deref() { rhs.deref(false) } else { rhs }
                        ).with_ty(ty)
                    }
                };
                Expr::Call(ci, arg_exprs).with_ty(ty)
            }
            ExprKind::Call { .. } => unreachable!(),
            ExprKind::Borrow { arg, .. } => {
                let inner_expr = self.expr_term(arg);
                if let TyKind::Ref(_, inner_ty, _) = ty.kind() {
                    if !inner_ty.peel_refs().to_string().starts_with("russol_contracts::Snapshot") { assert_eq!(*inner_ty, inner_expr.ty()) };
                    inner_expr.borrow(ty)
                } else { unreachable!() }
            }
            ExprKind::Adt(box AdtExpr { adt_def, variant_index, ref fields, .. }) => {
                self.ast_nodes += 1;
                let fields =
                    fields.iter().map(|f| (f.name, self.expr_term(f.expr))).collect();
                Expr::Constructor(self.tcx.item_name(adt_def.did()), variant_index, fields).with_ty(ty)
            }
            ExprKind::Deref { arg } => {
                if self.thir[arg].ty.is_ref() || self.thir[arg].ty.is_box() {
                    let is_fut = expr.span.macro_backtrace().last().map(|ed: rustc_span::ExpnData| ed.macro_def_id.map(|id| self.tcx.crate_name(id.krate).to_string() == "russol_macros").unwrap_or(false)).unwrap_or(false);
                    let e = self.expr_term(arg);
                    // println!("{:?} is from_expansion: {}", expr.span.macro_backtrace().last(), is_fut);
                    if is_fut {
                        self.ast_nodes += 1;
                    }
                    e.deref(is_fut)
                } else { todo!() }
            }
            ExprKind::Match { scrutinee, ref arms } => {
                self.ast_nodes += 1;
                let scrutinee = self.expr_term(scrutinee);
                arms.iter().map(|arm| self.arm_term(*arm, scrutinee.clone())).rev().reduce(|acc, arm| {
                    let (acc, arm) = (acc, arm);
                    (acc.0, Expr::IfElse(box arm.0, box arm.1, box acc.1).with_ty(ty))
                }).unwrap().1
            }
            ExprKind::Let { expr, ref pat } => {
                self.ast_nodes += 1;
                let expr = self.expr_term(expr);
                self.pattern_term(pat, expr)
            }
            ExprKind::If { cond, then, else_opt, .. } => {
                self.ast_nodes += 1;
                let cond = self.expr_term(cond);
                let then = self.expr_term(then);
                let els = if let Some(els) = else_opt {
                    self.expr_term(els)
                } else {
                    panic!("If expressions must have else!")
                };
                Expr::IfElse(box cond, box then, box els).with_ty(ty)
            }
            ExprKind::Field { lhs, name, variant_index } => {
                self.ast_nodes += 1;
                let lhs = self.expr_term(lhs);
                // println!("\nlhs.ty(): {}\n", lhs.ty());
                assert!(!lhs.ty().is_ref());
                lhs.field(variant_index, name, ty)
            }
            ExprKind::Array { ref fields } |
            ExprKind::Tuple { ref fields } => {
                self.ast_nodes += 1;
                let fields: Vec<_> =
                    fields.iter().enumerate().map(|(i, f)| (Field::from_usize(i), self.expr_term(*f))).collect();
                Expr::Constructor(AdtIdent::intern(&format!("tuple_{}", fields.len())), VariantIdx::from_u32(0), fields).with_ty(ty)
            }
            ExprKind::Cast { source } => {
                self.ast_nodes += 1;
                let expr = self.expr_term(source);
                expr.get_kind().with_ty(ty)
            }
            ExprKind::Use { source } => self.expr_term(source),
            ExprKind::NeverToAny { .. } => Expr::Never.with_ty(ty),
            ExprKind::ValueTypeAscription { source, .. } => self.expr_term(source),
            ExprKind::Box { value } => {
                self.ast_nodes += 1;
                self.expr_term(value)
            }
            ExprKind::Pointer { cast: PointerCast::Unsize, source } => self.expr_term(source),
            ExprKind::NamedConst { def_id, substs, .. } => {
                self.ast_nodes += 1;
                translate_constant(self.tcx, def_id, substs, ty)
            }
            ref ek => todo!("lower_expr: {:?}", ek),
        }
    }

    fn arm_term(
        &mut self,
        arm: ArmId,
        root: PureExpression<'tcx>,
    ) -> (PureExpression<'tcx>, PureExpression<'tcx>) {
        let arm = &self.thir[arm];

        let pattern = self.pattern_term(&arm.pattern, root);
        let guard = match arm.guard {
            Some(Guard::If(guard)) => self.expr_term(guard),
            Some(Guard::IfLet(ref pat, expr)) => {
                let expr = self.expr_term(expr);
                self.pattern_term(pat, expr)
            }
            None => PureExpression::from_bool(true, self.tcx),
        };
        let body = self.expr_term(arm.body);

        (pattern & guard, body)
    }

    fn pattern_term(
        &mut self,
        pat: &Pat<'tcx>,
        mut root: PureExpression<'tcx>,
    ) -> PureExpression<'tcx> {
        self.ast_nodes += 1;
        // trace!("{:?}", pat);
        let tcx = self.tcx;
        #[allow(unused_variables)]
        match &pat.kind {
            PatKind::Wild => PureExpression::from_bool(true, self.tcx),
            PatKind::Binding {
                name,
                var,
                subpattern,
                mode,
                ty,
                ..
            } => {
                let guard = if let Some(subpattern) = subpattern {
                    self.pattern_term(subpattern, root.clone())
                } else {
                    PureExpression::from_bool(true, self.tcx)
                };
                // println!("\nname: {}, mode: {:?}\n", name, mode);
                if let BindingMode::ByRef(_) = mode {
                    root = root.borrow(*ty);
                }
                self.var_map.insert(var.0, root);
                guard
            }
            PatKind::Variant {
                subpatterns,
                adt_def,
                variant_index,
                ..
            } => {
                let disc = adt_def.discriminant_for_variant(self.tcx, *variant_index);
                subpatterns
                    .iter()
                    .map(|pat| {
                        self.pattern_term(
                            &pat.pattern,
                            root.clone()
                                .field(*variant_index, pat.field, pat.pattern.ty),
                        )
                    })
                    .fold(root.clone().disc(disc, tcx), |acc, f| acc & f)
                // Ok(Pattern::Constructor { adt: adt_def, variant: *variant_index, fields })
            }
            PatKind::Leaf { subpatterns } => subpatterns
                .iter()
                .map(|pat| {
                    self.pattern_term(
                        &pat.pattern,
                        root.clone().field(0_u32.into(), pat.field, pat.pattern.ty),
                    )
                })
                .fold(PureExpression::from_bool(true, tcx), |acc, f| acc & f),
            PatKind::Deref { subpattern } => {
                // assert!(
                //     pat.ty.is_box() || pat.ty.ref_mutability() == Some(Mutability::Not),
                //     "pattern_term: only dereference over a box or shared reference is supported"
                // );
                self.pattern_term(subpattern, root.deref(false))
            }
            PatKind::Constant { value } => {
                let value = try_to_bits(
                    self.tcx,
                    self.tcx.param_env(self.item_id),
                    root.ty(),
                    *value,
                );
                value._eq(root, self.tcx)
            }
            PatKind::AscribeUserType {
                ascription,
                subpattern,
            } => todo!(),
            PatKind::Range(_) => todo!(),
            PatKind::Slice {
                prefix,
                slice,
                suffix,
            } => todo!(),
            PatKind::Array {
                prefix,
                slice,
                suffix,
            } => todo!(),
            PatKind::Or { pats } => pats
                .iter()
                .map(|pat| self.pattern_term(pat, root.clone()))
                .fold(PureExpression::from_bool(false, tcx), |acc, f| acc | f), // ref pk => todo!("lower_pattern: unsupported pattern kind {:?}", pk),
        }
    }

    fn stmt_term(&mut self, stmt: StmtId) {
        match self.thir[stmt].kind {
            StmtKind::Expr { expr, .. } => {
                if let ExprKind::Scope { value, .. } = self.thir[expr].kind {
                    if let ExprKind::Call { ty, .. } = self.thir[value].kind {
                        if let TyKind::FnDef(id, _) = ty.kind() {
                            if self.tcx.def_path_str(*id) == "russol_contracts::trusted_ensures" {
                                return;
                            }
                        }
                    }
                }
                panic!("redundant expr in block {:?}", self.thir[expr].span);
            }
            StmtKind::Let {
                ref pattern,
                initializer,
                init_scope,
                ..
            } => {
                if let Some(initializer) = initializer {
                    let initializer = self.expr_term(initializer);
                    self.pattern_term(pattern, initializer);
                } else {
                    let span = self.tcx.hir().span(HirId {
                        owner: self.item_id,
                        local_id: init_scope.id,
                    });
                    panic!("let-bindings must have values {span:?}");
                }
            }
        }
    }

    fn get_stub_kind(&self, id: DefId, substs: SubstsRef) -> Option<RuslikStub> {
        if self.is_id_special(id) {
            let fn_name = self.tcx.def_path_str(id);
            if fn_name == "russol_contracts::Snapshotable::snap" {
                Some(RuslikStub::Snap)
            } else if fn_name.starts_with("russol_contracts::Set::") && fn_name.ends_with("::new") {
                Some(RuslikStub::SetNew)
            } else {
                todo!("Unsupported builtin fn encountered: {}", fn_name)
            }
        } else if substs.types().any(|ty| {
            if let TyKind::Adt(def, _) = ty.kind() {
                self.is_id_special(def.did())
            } else {
                false
            }
        }) {
            let fn_name = self.tcx.def_path_str(id);
            if fn_name == "std::ops::Add::add" {
                Some(RuslikStub::Add)
            } else if fn_name == "std::ops::Sub::sub" {
                Some(RuslikStub::Sub)
            } else if fn_name == "std::ops::Index::index" {
                Some(RuslikStub::In)
            } else if fn_name == "std::cmp::PartialEq::eq" {
                Some(RuslikStub::Eq)
            } else if fn_name == "std::cmp::PartialOrd::ge" {
                Some(RuslikStub::Ge)
            } else if fn_name == "std::cmp::PartialOrd::le" {
                Some(RuslikStub::Le)
            } else if fn_name == "std::cmp::PartialOrd::gt" {
                Some(RuslikStub::Gt)
            } else if fn_name == "std::cmp::PartialOrd::lt" {
                Some(RuslikStub::Lt)
            } else {
                todo!("Unsupported builtin fn encountered: {}", fn_name)
            }
        } else {
            None
        }
    }
    fn is_id_special(&self, id: DefId) -> bool {
        self.tcx.crate_name(id.krate).to_string() == "russol_contracts"
    }
}

#[derive(Clone, Copy)]
enum RuslikStub {
    Snap,
    SetNew,
    Add,
    Sub,
    Eq,
    Ge,
    Le,
    Gt,
    Lt,
    In,
}
impl RuslikStub {
    fn to_binop(self) -> rustc_hir::BinOpKind {
        match self {
            RuslikStub::Add => rustc_hir::BinOpKind::Add,
            RuslikStub::Sub => rustc_hir::BinOpKind::Sub,
            RuslikStub::Eq => rustc_hir::BinOpKind::Eq,
            RuslikStub::Ge => rustc_hir::BinOpKind::Ge,
            RuslikStub::Le => rustc_hir::BinOpKind::Le,
            RuslikStub::Gt => rustc_hir::BinOpKind::Gt,
            RuslikStub::Lt => rustc_hir::BinOpKind::Lt,
            RuslikStub::In | RuslikStub::Snap | RuslikStub::SetNew => panic!(),
        }
    }
    fn expect_deref(self) -> bool {
        match self {
            RuslikStub::Add | RuslikStub::Sub => false,
            RuslikStub::Eq | RuslikStub::Ge | RuslikStub::Le | RuslikStub::Gt | RuslikStub::Lt => {
                true
            }
            RuslikStub::In | RuslikStub::Snap | RuslikStub::SetNew => panic!(),
        }
    }
}
