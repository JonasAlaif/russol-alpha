use std::rc::Rc;

use rustc_infer::infer::outlives::env::OutlivesEnvironment;
use rustc_middle::ty::{Ty, TyCtxt, TyKind};
use rustc_span::def_id::DefId;
use rustc_target::abi::VariantIdx;

use crate::{
    ruslik_pure,
    ruslik_ssl::{self, Var},
};

pub(crate) type AdtIdent = rustc_span::symbol::Symbol;
pub(crate) type FieldIdent = rustc_middle::mir::Field;

pub(crate) const DISC: (FieldIdent, VariantIdx) = (FieldIdent::MAX, VariantIdx::MAX);
pub(crate) const FUT: (FieldIdent, VariantIdx) = (
    FieldIdent::from_u32(FieldIdent::MAX_AS_U32 - 1),
    VariantIdx::from_u32(0),
);
pub(crate) const OLD: (FieldIdent, VariantIdx) = (FieldIdent::MAX, VariantIdx::from_u32(0));

pub(crate) fn field_to_name(f: FieldIdent, v: VariantIdx, ty: Ty) -> String {
    // println!(" [TY: {}] ", ty);
    match (f, v) {
        DISC => "disc".to_string(),
        OLD => "*".to_string(),
        FUT => "^".to_string(),
        _ => match ty.kind() {
            TyKind::Adt(adt, _) => adt.variant(v).fields[f.index()].name.as_str().to_string(),
            TyKind::Tuple(_) => format!("_{}", f.as_u32()),
            _ => todo!(),
        },
    }
}

#[derive(Clone)]
pub struct RuslikFnSig<'tcx> {
    pub args: Vec<(ruslik_ssl::Var, Ty<'tcx>)>,
    pub ret: Ty<'tcx>,
    pub pure_pre: ruslik_pure::PureExpression<'tcx>,
    pub pure_post: ruslik_pure::PureExpression<'tcx>,
    pub def_id: DefId,
    pub outlives: Rc<OutlivesEnvironment<'tcx>>,
    pub params: String,
    pub ast_nodes: usize,
}
impl<'tcx> RuslikFnSig<'tcx> {
    pub(crate) fn new(
        def_id: DefId,
        tcx: TyCtxt<'tcx>,
        pure_pre: ruslik_pure::PureExpression<'tcx>,
        pure_post: ruslik_pure::PureExpression<'tcx>,
        params: String,
        ast_nodes: usize,
    ) -> Self {
        // println!("{:?}", self.tcx.named_region_map(def_id.expect_local()));
        // println!("{:?}", def_id.expect_local());
        if let Some(local) = def_id.as_local() {
            let owner = rustc_hir::HirId::make_owner(local);
            let _sig = tcx.hir().fn_sig_by_hir_id(owner).unwrap();
            // println!("{:?}", sig.decl.inputs.into_iter().map(|p| p).collect::<Vec<_>>());
        }

        let gen_sig = tcx.fn_sig(def_id);
        // println!("{:?}", gen_sig.inputs_and_output());

        let outlives = Rc::new(OutlivesEnvironment::new(tcx.param_env(def_id)));
        // println!("{:?}", outlives.free_region_map());
        // let sig = self
        //     .tcx
        //     .normalize_erasing_late_bound_regions(self.tcx.param_env(def_id), gen_sig);
        let sig: rustc_middle::ty::FnSig = tcx.liberate_late_bound_regions(def_id, gen_sig);
        // println!("Input and output types of {:?}: {:?}", def_id, sig.inputs_and_output.iter().map(|ty| ty).collect::<Vec<_>>());
        let args = tcx
            .fn_arg_names(def_id)
            .iter()
            .map(|id| Var::arg(id.name))
            .zip(sig.inputs().iter().copied())
            .collect();
        RuslikFnSig {
            args,
            ret: sig.output(),
            pure_pre,
            pure_post,
            def_id,
            outlives,
            params,
            ast_nodes,
        }
    }

    pub fn is_trivial(&self) -> bool {
        let return_trivial = self.ret.is_unit() || self.ret.is_primitive();
        self.pure_post.is_true() && return_trivial
    }
}
