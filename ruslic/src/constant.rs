use rustc_hir::def_id::DefId;
use rustc_middle::{
    mir::ConstantKind,
    ty::{self, Const, ConstKind, ParamEnv, SubstsRef, Ty, TyCtxt},
};

use crate::ruslik_pure::PureExpression;

pub(crate) fn translate_constant<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    subst: SubstsRef<'tcx>,
    ty: Ty<'tcx>,
) -> PureExpression<'tcx> {
    let uneval = ty::Unevaluated::new(ty::WithOptConstParam::unknown(def_id), subst);
    let constant = tcx.mk_const(ty::ConstS {
        kind: ty::ConstKind::Unevaluated(uneval),
        ty,
    });

    from_ty_const(tcx, constant, tcx.param_env(def_id))
}

fn from_ty_const<'tcx>(
    tcx: TyCtxt<'tcx>,
    c: Const<'tcx>,
    env: ParamEnv<'tcx>,
) -> PureExpression<'tcx> {
    if let ConstKind::Param(_) = c.kind() {
        todo!("const generic parameters are not yet supported");
    }
    try_to_bits(tcx, env, c.ty(), c)
}

pub(crate) fn try_to_bits<'tcx, C: ToBits<'tcx>>(
    tcx: TyCtxt<'tcx>,
    env: ParamEnv<'tcx>,
    ty: Ty<'tcx>,
    c: C,
) -> PureExpression<'tcx> {
    use rustc_middle::ty::{FloatTy, IntTy, UintTy};
    use rustc_type_ir::sty::TyKind::{Bool, Float, Int, Uint};
    match ty.kind() {
        Int(ity) => {
            let bits = c.get_bits(tcx, env, ty).unwrap();
            let bits: i128 = match *ity {
                IntTy::I128 => bits as i128,
                IntTy::Isize => bits as i64 as i128,
                IntTy::I8 => bits as i8 as i128,
                IntTy::I16 => bits as i16 as i128,
                IntTy::I32 => bits as i32 as i128,
                IntTy::I64 => bits as i64 as i128,
            };
            let expr = PureExpression::from_u128(bits.unsigned_abs(), ty);
            if bits < 0 {
                -expr
            } else {
                expr
            }
        }
        Uint(uty) => {
            let bits = c.get_bits(tcx, env, ty).unwrap();
            let bits: u128 = match *uty {
                UintTy::U128 => bits as u128,
                UintTy::Usize => bits as u64 as u128,
                UintTy::U8 => bits as u8 as u128,
                UintTy::U16 => bits as u16 as u128,
                UintTy::U32 => bits as u32 as u128,
                UintTy::U64 => bits as u64 as u128,
            };
            PureExpression::from_u128(bits, ty)
        }
        Bool => PureExpression::from_bool(c.get_bits(tcx, env, ty) == Some(1), tcx),
        Float(FloatTy::F32) => {
            let bits = c.get_bits(tcx, env, ty);
            let float = f32::from_bits(bits.unwrap() as u32);
            todo!("Floats are not yet supported ({float})")
        }
        Float(FloatTy::F64) => {
            let bits = c.get_bits(tcx, env, ty);
            let float = f64::from_bits(bits.unwrap() as u64);
            todo!("Floats are not yet supported ({float})")
        }
        _ => todo!("Unsupported constant of type {ty}"),
    }
}

pub(crate) trait ToBits<'tcx> {
    fn get_bits(&self, tcx: TyCtxt<'tcx>, env: ParamEnv<'tcx>, ty: Ty<'tcx>) -> Option<u128>;
}

impl<'tcx> ToBits<'tcx> for Const<'tcx> {
    fn get_bits(&self, tcx: TyCtxt<'tcx>, env: ParamEnv<'tcx>, ty: Ty<'tcx>) -> Option<u128> {
        self.try_eval_bits(tcx, env, ty)
    }
}
impl<'tcx> ToBits<'tcx> for ConstantKind<'tcx> {
    fn get_bits(&self, tcx: TyCtxt<'tcx>, env: ParamEnv<'tcx>, ty: Ty<'tcx>) -> Option<u128> {
        self.try_eval_bits(tcx, env, ty)
    }
}
