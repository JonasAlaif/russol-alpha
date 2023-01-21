use std::{fmt, ops::Neg};

use rustc_hir::def_id::DefId;
use rustc_middle::{
    mir::Field,
    ty::{Ty, TyCtxt},
};
use rustc_middle::{
    mir::UnOp,
    ty::{subst::SubstsRef, util::Discr},
};
use rustc_target::abi::VariantIdx;

use crate::ruslik_types::{field_to_name, DISC, FUT, OLD};
use crate::{ruslik_ssl::Var, ruslik_types::AdtIdent};

#[derive(Debug, Copy, Clone)]
pub enum UnOpKind {
    Snap,
    UnOp(UnOp),
}
#[derive(Debug, Clone, Copy)]
pub enum CallInfo<'tcx> {
    Pure(DefId, SubstsRef<'tcx>),
    Builtin(BuiltinCallKind),
}
#[derive(Debug, Clone, Copy)]
pub enum BuiltinCallKind {
    SetConstruct,
    SetContains,
}
type BinOp = rustc_hir::BinOpKind;
type Lit = rustc_ast::ast::LitKind;

#[derive(Debug, Clone)]
pub struct PureExpression<'tcx> {
    ty: Ty<'tcx>,
    kind: ExprKind<'tcx>,
}

#[derive(Debug, Clone)]
pub enum ExprKind<'tcx> {
    Never,
    Var(Var),
    // Debrujin(usize),
    Lit(Lit),
    BinOp(BinOp, Box<PureExpression<'tcx>>, Box<PureExpression<'tcx>>),
    UnOp(UnOpKind, Box<PureExpression<'tcx>>),
    // Constructor:
    Constructor(AdtIdent, VariantIdx, Vec<(Field, PureExpression<'tcx>)>),
    // Destructor:
    Field(Box<PureExpression<'tcx>>, VariantIdx, Field),
    // Match(Box<PureExpression<'tcx>>, Vec<MatchArm>),
    IfElse(
        Box<PureExpression<'tcx>>,
        Box<PureExpression<'tcx>>,
        Box<PureExpression<'tcx>>,
    ),
    Call(CallInfo<'tcx>, Vec<PureExpression<'tcx>>),
}
impl<'tcx> ExprKind<'tcx> {
    pub fn with_ty(self, ty: Ty<'tcx>) -> PureExpression<'tcx> {
        PureExpression { ty, kind: self }
    }
}
impl<'tcx> PureExpression<'tcx> {
    pub fn ty(&self) -> Ty<'tcx> {
        self.ty
    }
    pub fn kind(&self) -> &ExprKind<'tcx> {
        &self.kind
    }
    pub fn ty_mut(&mut self) -> &mut Ty<'tcx> {
        &mut self.ty
    }
    pub fn kind_mut(&mut self) -> &mut ExprKind<'tcx> {
        &mut self.kind
    }
    pub fn get_kind(self) -> ExprKind<'tcx> {
        self.kind
    }
    pub fn is_result(&self) -> bool {
        matches!(&self.kind, ExprKind::Var(v) if v.uuid() == "result")
    }
    pub fn is_true(&self) -> bool {
        matches!(&self.kind, ExprKind::Lit(Lit::Bool(true)))
    }
    pub fn from_u32(item: u32, tcx: TyCtxt<'tcx>) -> Self {
        ExprKind::Lit(Lit::Int(
            item.into(),
            rustc_ast::ast::LitIntType::Unsuffixed,
        ))
        .with_ty(tcx.types.u32)
    }
    pub fn from_u64(item: u64, tcx: TyCtxt<'tcx>) -> Self {
        ExprKind::Lit(Lit::Int(
            item.into(),
            rustc_ast::ast::LitIntType::Unsuffixed,
        ))
        .with_ty(tcx.types.u64)
    }
    pub fn from_u128(item: u128, ty: Ty<'tcx>) -> Self {
        ExprKind::Lit(Lit::Int(item, rustc_ast::ast::LitIntType::Unsuffixed)).with_ty(ty)
    }
    pub fn from_i64(item: i64, tcx: TyCtxt<'tcx>) -> Self {
        // TODO: fix ty
        let lit = Self::from_u64(item.abs().try_into().unwrap(), tcx)
            .kind
            .with_ty(tcx.types.i32);
        if item < 0 {
            -lit
        } else {
            lit
        }
    }
    pub fn from_bool(item: bool, tcx: TyCtxt<'tcx>) -> Self {
        ExprKind::Lit(Lit::Bool(item)).with_ty(tcx.types.bool)
    }
    pub fn _eq(self, other: Self, tcx: TyCtxt<'tcx>) -> Self {
        assert!(self.ty == other.ty);
        ExprKind::BinOp(BinOp::Eq, box self, box other).with_ty(tcx.types.bool)
    }
    pub fn _ge(self, other: Self, tcx: TyCtxt<'tcx>) -> Self {
        assert!(self.ty == other.ty);
        ExprKind::BinOp(BinOp::Ge, box self, box other).with_ty(tcx.types.bool)
    }
    pub fn borrow(self, ty: Ty<'tcx>) -> Self {
        let (f, v) = OLD;
        ExprKind::Constructor(AdtIdent::intern("&"), v, vec![(f, self)]).with_ty(ty)
    }
    pub fn deref(mut self, fut: bool) -> Self {
        // Check safety and get inner type
        if self.ty.is_box() {
            assert!(!fut, "Cannot use `^` on Box type expression `{self}`!")
        }
        let ty = if let Some(ty_mut) = self.ty.builtin_deref(false) {
            ty_mut.ty
        } else {
            panic!("Cannot deref expr `{self}`!")
        };
        // Get variant and field indicies
        let (f, v) = if fut { FUT } else { OLD };
        if let ExprKind::Constructor(adt_id, _, fields) = &mut self.kind && adt_id.as_str() == "&" {
            let expr = fields.pop().unwrap().1;
            assert!(!fut, "Encountered disallowed `^&...` in expression: ^&{}", expr);
            expr
        } else {
            ExprKind::Field(box self, v, f).with_ty(ty)
        }
    }
    pub fn field(self, v: VariantIdx, f: Field, ty: Ty<'tcx>) -> Self {
        let new_self = self;
        if let ExprKind::Constructor(_, vid, fields) = new_self.kind {
            assert!(v == vid);
            return fields.into_iter().find(|(fd, _)| *fd == f).unwrap().1;
        }
        ExprKind::Field(box new_self, v, f).with_ty(ty)
    }
    pub fn disc(self, disc: Discr<'tcx>, tcx: TyCtxt<'tcx>) -> Self {
        let variant = Self::from_u128(disc.val, disc.ty);
        let (f, v) = DISC;
        self.field(v, f, variant.ty())._eq(variant, tcx)
    }
}
impl<'tcx> fmt::Display for PureExpression<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            ExprKind::Never => todo!(),
            ExprKind::Var(v) => write!(f, "{}", v.rname()),
            // Self::Debrujin(d) => write!(f, "!{}", d),
            ExprKind::Lit(Lit::Bool(b)) => write!(f, "{}", b),
            ExprKind::Lit(Lit::Int(i, rustc_ast::ast::LitIntType::Unsuffixed)) => {
                write!(f, "{}", i)
            }
            ExprKind::Lit(_) => todo!("{:?}", self),
            ExprKind::UnOp(UnOpKind::UnOp(UnOp::Not), e) => write!(f, "(! {})", e),
            ExprKind::UnOp(UnOpKind::UnOp(UnOp::Neg), e) => write!(f, "(- {})", e),
            ExprKind::UnOp(UnOpKind::Snap, e) => write!(f, "(@ {})", e),
            ExprKind::BinOp(op, box l, box r) => write!(f, "({} {} {})", l, op.as_str(), r),
            ExprKind::Constructor(id, _, fds) if id.as_str().starts_with("tuple_") => write!(
                f,
                "{}({})",
                id,
                fds.iter()
                    .map(|(_, expr)| expr.to_string())
                    .intersperse(", ".to_string())
                    .collect::<String>()
            ),
            ExprKind::Constructor(id, v, fds) => write!(
                f,
                "{}_{}{{ {} }}",
                id,
                v.as_u32(),
                fds.iter()
                    .map(|(fd, expr)| field_to_name(*fd, *v, self.ty()) + ": " + &expr.to_string())
                    .intersperse(", ".to_string())
                    .collect::<String>()
            ),
            ExprKind::Field(pe, v, field) => {
                write!(f, "{}.{}", pe, field_to_name(*field, *v, pe.ty))
            }
            ExprKind::IfElse(cond, te, fe) => {
                write!(f, "if {} {{ {} }} else {{ {} }}", cond, te, fe)
            }
            ExprKind::Call(ci, args) => write!(
                f,
                "[{:?}]({})",
                ci,
                args.iter()
                    .map(|arg| arg.to_string())
                    .intersperse(", ".to_string())
                    .collect::<String>()
            ),
        }
    }
}

impl<'tcx> Neg for PureExpression<'tcx> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        let ty = self.ty;
        ExprKind::UnOp(UnOpKind::UnOp(UnOp::Neg), box self).with_ty(ty)
    }
}
impl<'tcx> std::ops::BitAnd<PureExpression<'tcx>> for PureExpression<'tcx> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        assert!(self.ty == rhs.ty);
        let ty = self.ty;
        // TODO: check types and behave differently for boolean and int types
        match (&self.kind, &rhs.kind) {
            // Optimizations:
            (ExprKind::Lit(Lit::Bool(true)), _) | (_, ExprKind::Lit(Lit::Bool(false))) => rhs,
            (_, ExprKind::Lit(Lit::Bool(true))) | (ExprKind::Lit(Lit::Bool(false)), _) => self,
            // Constructor:
            _ => ExprKind::BinOp(BinOp::And, box self, box rhs).with_ty(ty),
        }
    }
}
impl<'tcx> std::ops::BitOr<PureExpression<'tcx>> for PureExpression<'tcx> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        assert!(self.ty == rhs.ty);
        let ty = self.ty;
        // TODO: check types and behave differently for boolean and int types
        match (&self.kind, &rhs.kind) {
            // Optimizations:
            (ExprKind::Lit(Lit::Bool(true)), _) | (_, ExprKind::Lit(Lit::Bool(false))) => self,
            (_, ExprKind::Lit(Lit::Bool(true))) | (ExprKind::Lit(Lit::Bool(false)), _) => rhs,
            // Constructor:
            _ => ExprKind::BinOp(BinOp::Or, box self, box rhs).with_ty(ty),
        }
    }
}
