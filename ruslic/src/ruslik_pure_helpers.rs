use rustc_middle::ty::{TypeFoldable, TypeFolder};

use crate::ruslik_pure::{CallInfo, ExprKind, PureExpression};

impl<'tcx> PureExpression<'tcx> {
    pub fn walk_mut<T: PureExpressionWalker<'tcx>>(&mut self, walker: &mut T) {
        walker.walk_kind_mut(self.kind_mut());
    }
}
impl<'tcx> ExprKind<'tcx> {
    pub fn walk_mut<T: PureExpressionWalker<'tcx>>(&mut self, walker: &mut T) {
        match self {
            ExprKind::Never | ExprKind::Var(_) | ExprKind::Lit(_) => (),
            ExprKind::BinOp(_, l, r) => {
                walker.walk_expr_mut(l);
                walker.walk_expr_mut(r);
            }
            ExprKind::UnOp(_, e) | ExprKind::Field(e, _, _) => walker.walk_expr_mut(e),
            ExprKind::Constructor(_, _, es) => {
                for e in es {
                    walker.walk_expr_mut(&mut e.1)
                }
            }
            ExprKind::IfElse(c, t, e) => {
                walker.walk_expr_mut(c);
                walker.walk_expr_mut(t);
                walker.walk_expr_mut(e);
            }
            ExprKind::Call(_, es) => {
                for e in es {
                    walker.walk_expr_mut(e)
                }
            }
        }
    }
}
pub trait PureExpressionWalker<'tcx>: Sized {
    fn walk_expr_mut(&mut self, e: &mut PureExpression<'tcx>) {
        e.walk_mut(self);
    }
    fn walk_kind_mut(&mut self, k: &mut ExprKind<'tcx>) {
        match k {
            ExprKind::Never | ExprKind::Var(_) | ExprKind::Lit(_) => (),
            ExprKind::BinOp(_, l, r) => {
                self.walk_expr_mut(l);
                self.walk_expr_mut(r);
            }
            ExprKind::UnOp(_, e) | ExprKind::Field(e, _, _) => self.walk_expr_mut(e),
            ExprKind::Constructor(_, _, es) => {
                for e in es {
                    self.walk_expr_mut(&mut e.1)
                }
            }
            ExprKind::IfElse(c, t, e) => {
                self.walk_expr_mut(c);
                self.walk_expr_mut(t);
                self.walk_expr_mut(e);
            }
            ExprKind::Call(CallInfo::Pure(_, _), _) => todo!(),
            ExprKind::Call(CallInfo::Builtin(_), es) => {
                for e in es {
                    self.walk_expr_mut(e)
                }
            }
        }
    }
}

impl<'tcx, F: TypeFolder<'tcx>> PureExpressionWalker<'tcx> for F {
    fn walk_expr_mut(&mut self, e: &mut PureExpression<'tcx>) {
        *e.ty_mut() = self.fold_ty(e.ty());
        e.walk_mut(self);
    }
    fn walk_kind_mut(&mut self, k: &mut ExprKind<'tcx>) {
        if let ExprKind::Call(CallInfo::Pure(_, gas), _) = k {
            *gas = gas.fold_with(self);
        }
        k.walk_mut(self);
    }
}
