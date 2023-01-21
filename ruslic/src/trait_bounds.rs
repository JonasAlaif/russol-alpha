use rustc_data_structures::fx::FxHashSet;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{
    AssocItems, AssocKind, EarlyBinder, GenericPredicates, PredicateKind, Subst, Ty, TyCtxt,
};

use crate::{ruslik_pure::PureExpression, ruslik_types::RuslikFnSig};

pub(crate) fn find_trait_fns<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    tys: &FxHashSet<Ty<'tcx>>,
) -> Vec<RuslikFnSig<'tcx>> {
    let mut gp: Vec<GenericPredicates> = vec![tcx.predicates_of(def_id)];
    while let Some(parent) = gp.last().unwrap().parent {
        gp.push(tcx.predicates_of(parent));
    }
    gp.into_iter()
        .flat_map(|gp| gp.predicates.iter())
        .flat_map(|(pred, _)| {
            let pred_kind = if let Some(pred_kind) = pred.kind().no_bound_vars() {
                pred_kind
            } else {
                // eprintln!("Binder has vars: {:?} ({:?})", pred.kind(), def_id);
                return Vec::new();
            };
            match pred_kind {
                PredicateKind::Trait(t) => {
                    if !tys.contains(&t.self_ty()) {
                        // eprintln!("Skipping predicate {t:?} since self {} isn't translated.", t.self_ty());
                        return Vec::new();
                    }
                    let items: &AssocItems = tcx.associated_items(t.trait_ref.def_id);
                    items
                        .in_definition_order()
                        .filter(|i| i.kind == AssocKind::Fn)
                        .flat_map(|method| {
                            // Ignore trait fns with generics
                            if !tcx.generics_of(method.def_id).params.is_empty() {
                                return None;
                            }
                            let pre = PureExpression::from_bool(true, tcx);
                            let post = PureExpression::from_bool(true, tcx);
                            let mut rfs =
                                RuslikFnSig::new(method.def_id, tcx, pre, post, String::new(), 0);
                            // Ignore fns with args like "(_: i32, Struct { f }: Struct)" since we don't support them yet
                            if rfs.args.iter().any(|(v, _)| v.uuid().is_empty()) {
                                return None;
                            }
                            rfs.args = rfs
                                .args
                                .into_iter()
                                .map(|(arg, ty)| {
                                    (arg, EarlyBinder(ty).subst(tcx, t.trait_ref.substs))
                                })
                                .collect();
                            // println!("Substituting at {:?} with ty {} and gens {:?}", method.def_id, rfs.ret.ty, t.trait_ref.substs);
                            rfs.ret = EarlyBinder(rfs.ret).subst(tcx, t.trait_ref.substs);
                            Some(rfs)
                        })
                        .collect()
                }
                PredicateKind::RegionOutlives(_) | PredicateKind::TypeOutlives(_) => Vec::new(),
                PredicateKind::Projection(_)
                | PredicateKind::WellFormed(_)
                | PredicateKind::ObjectSafe(_)
                | PredicateKind::ClosureKind(_, _, _)
                | PredicateKind::Subtype(_)
                | PredicateKind::Coerce(_)
                | PredicateKind::ConstEvaluatable(_)
                | PredicateKind::ConstEquate(_, _)
                | PredicateKind::TypeWellFormedFromEnv(_) => {
                    // eprintln!("Skipping Predicate: {:?}", pred_kind);
                    Vec::new()
                }
            }
        })
        .collect()
}
