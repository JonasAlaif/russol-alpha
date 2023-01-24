use std::ops::ControlFlow;

use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_middle::ty::{
    GenericParamDef, ParamTy, SubstsRef, Ty, TyCtxt, TyKind, TypeFolder, TypeSuperFoldable,
    TypeVisitable, TypeVisitor,
};

use crate::{
    ruslik_pure::PureExpression, ruslik_pure_helpers::PureExpressionWalker,
    ruslik_types::RuslikFnSig,
};

pub struct SubstFolder<'a, 'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub subst: Box<dyn Fn(ParamTy) -> Ty<'tcx> + 'a>,
}
impl<'a, 'tcx> TypeFolder<'tcx> for SubstFolder<'a, 'tcx> {
    fn tcx<'b>(&'b self) -> TyCtxt<'tcx> {
        self.tcx
    }
    fn fold_ty(&mut self, t: Ty<'tcx>) -> Ty<'tcx> {
        if !t.needs_subst() {
            t
        } else {
            match *t.kind() {
                TyKind::Param(p) => (self.subst)(p),
                _ => t.super_fold_with(self),
            }
        }
    }
}
impl<'a, 'tcx: 'a> SubstFolder<'a, 'tcx> {
    pub fn from_gens(tcx: TyCtxt<'tcx>, substs: &'a FxHashMap<u32, Ty<'tcx>>) -> Self {
        SubstFolder {
            tcx,
            subst: Box::new(|p| substs[&p.index]),
        }
    }
    pub fn from_substs_ref(tcx: TyCtxt<'tcx>, substs: SubstsRef<'tcx>) -> Self {
        SubstFolder {
            tcx,
            subst: Box::new(|p| substs.type_at(p.index as usize)),
        }
    }
}

pub trait TyFoldable<'tcx> {
    fn subst<F: TypeFolder<'tcx>>(&mut self, folder: &mut F);
}

impl<'tcx> TyFoldable<'tcx> for Ty<'tcx> {
    fn subst<F: TypeFolder<'tcx>>(&mut self, folder: &mut F) {
        *self = folder.fold_ty(*self)
    }
}

impl<'tcx> TyFoldable<'tcx> for PureExpression<'tcx> {
    fn subst<F: TypeFolder<'tcx>>(&mut self, folder: &mut F) {
        folder.walk_expr_mut(self);
    }
}

impl<'tcx> TyFoldable<'tcx> for RuslikFnSig<'tcx> {
    fn subst<F: TypeFolder<'tcx>>(&mut self, folder: &mut F) {
        for (_, rt) in &mut self.args {
            *rt = folder.fold_ty(*rt);
        }
        self.ret = folder.fold_ty(self.ret);
        folder.walk_expr_mut(&mut self.pure_pre);
        folder.walk_expr_mut(&mut self.pure_post);
    }
}

pub struct SubstFinder<'tcx> {
    pub param_subs: FxHashMap<u32, Ty<'tcx>>,
}
impl<'tcx> SubstFinder<'tcx> {
    fn new() -> Self {
        Self {
            param_subs: FxHashMap::default(),
        }
    }
    fn visit_ty_tuple(&mut self, l: Ty<'tcx>, r: Ty<'tcx>) -> Result<(), ()> {
        match (l.kind(), r.kind()) {
            (TyKind::Adt(l, ls), TyKind::Adt(r, rs)) if l == r => {
                assert_eq!(ls.len(), rs.len());
                // TODO check and subst regions?
                for (l, r) in ls.types().zip(rs.types()) {
                    self.visit_ty_tuple(l, r)?;
                }
                Ok(())
            }
            (TyKind::Ref(_, l, _), TyKind::Ref(_, r, _)) => self.visit_ty_tuple(*l, *r),
            (TyKind::Tuple(l), TyKind::Tuple(r)) if l.len() == r.len() => {
                for (l, r) in l.iter().zip(r.iter()) {
                    self.visit_ty_tuple(l, r)?;
                }
                Ok(())
            }
            (TyKind::Param(p), _) => {
                if let Some(ty) = self.param_subs.insert(p.index, r) {
                    if ty != r {
                        return Err(());
                    }
                }
                Ok(())
            }
            (a, b) => {
                if a == b {
                    Ok(())
                } else {
                    Err(())
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct VecIter {
    curr: Vec<usize>,
    started: bool,
    start: usize,
    end: usize,
}
impl VecIter {
    pub fn new(start: usize, end: usize, width: usize) -> Self {
        VecIter {
            curr: vec![end; width],
            started: false,
            start,
            end,
        }
    }
    pub fn next<'a>(&'a mut self) -> Option<&'a Vec<usize>> {
        for i in &mut self.curr {
            if *i + 1 >= self.end {
                *i = self.start;
            } else {
                *i += 1;
                return Some(&self.curr);
            }
        }
        if self.started {
            self.end = self.start;
            None
        } else {
            self.started = true;
            Some(&self.curr)
        }
    }
    pub fn total_elems(&self) -> usize {
        let res = (self.end - self.start) * self.curr.len();
        if res == 0 {
            1
        } else {
            res
        }
    }
}
#[derive(Debug)]
pub struct VecIter2 {
    curr: Vec<usize>,
    max: Vec<usize>,
    started: bool,
}
impl VecIter2 {
    pub fn new(max: Vec<usize>) -> Self {
        if max.iter().any(|v| *v == 0) {
            VecIter2 {
                curr: Vec::new(),
                max: Vec::new(),
                started: true,
            }
        } else {
            VecIter2 {
                curr: vec![0; max.len()],
                max,
                started: false,
            }
        }
    }
    pub fn next<'a>(&'a mut self) -> Option<&'a Vec<usize>> {
        for (i, max) in &mut self.curr.iter_mut().zip(self.max.iter()) {
            if !self.started {
                self.started = true;
                return Some(&self.curr);
            }
            if *i + 1 >= *max {
                *i = 0;
            } else {
                *i += 1;
                return Some(&self.curr);
            }
        }
        if !self.started {
            self.started = true;
            Some(&self.curr)
        } else {
            None
        }
    }
    pub fn total_elems(&self) -> usize {
        if self.max.is_empty() {
            0
        } else {
            self.max.iter().product()
        }
    }
}

pub struct SGenericsCollector<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub synth_tys: FxHashSet<Ty<'tcx>>,
}
impl<'tcx> SGenericsCollector<'tcx> {
    pub fn find_subs_for_ext_fns(
        &self,
        extern_fn: &RuslikFnSig<'tcx>,
    ) -> Vec<(String, RuslikFnSig<'tcx>)> {
        // Collect generics
        let mut generics_of = self.tcx.generics_of(extern_fn.def_id);
        let param_count = generics_of.parent_count + generics_of.params.len();
        let mut params: Vec<&GenericParamDef> = generics_of.params.iter().collect();
        while let Some(parent) = generics_of.parent {
            generics_of = self.tcx.generics_of(parent);
            params.extend(&generics_of.params);
        }
        assert_eq!(param_count, params.len());

        //
        // let param_to_idx: FxHashMap<_, _> = params.iter().filter(|p| matches!(p.kind, GenericParamDefKind::Type { .. })).enumerate().map(|(i, p)| (p.index, i)).collect();
        let efn_tys: Vec<_> = extern_fn
            .args
            .iter()
            .map(|(_, ty)| *ty)
            .chain(std::iter::once(extern_fn.ret))
            .collect();

        let mut possible_subs = FxHashMap::default();
        for efn_ty in &efn_tys {
            struct ParamCollector<'tcx> {
                params: FxHashMap<u32, FxHashSet<Ty<'tcx>>>,
            }
            impl<'tcx> TypeVisitor<'tcx> for ParamCollector<'tcx> {
                fn visit_ty(&mut self, t: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {
                    use rustc_middle::ty::TypeSuperVisitable;
                    t.super_visit_with(self);
                    if let TyKind::Param(p) = t.kind() {
                        self.params.insert(p.index, FxHashSet::default());
                    }
                    ControlFlow::Continue(())
                }
            }
            let mut pc = ParamCollector {
                params: FxHashMap::default(),
            };
            pc.visit_ty(*efn_ty);
            // TODO: decide on this flag
            let all_possible_subs = false;
            if !all_possible_subs {
                for synth_ty in &self.synth_tys {
                    let mut sf = SubstFinder::new();
                    // Peel refs for fn arg since these can be created (just borrow to create arg)
                    let (mut efn_ty, mut synth_ty) = (*efn_ty, *synth_ty);
                    while let TyKind::Ref(_, inner_ty, _) = efn_ty.kind() {
                        efn_ty = *inner_ty;
                        if let TyKind::Ref(_, inner_ty, _) = synth_ty.kind() {
                            synth_ty = *inner_ty;
                        }
                    }
                    if let Ok(()) = sf.visit_ty_tuple(efn_ty, synth_ty) {
                        for (param, ty) in sf.param_subs {
                            pc.params.get_mut(&param).unwrap().insert(ty);
                        }
                    }
                }
            }
            for (param, tys) in pc.params {
                if all_possible_subs {
                    possible_subs
                        .entry(param)
                        .or_insert_with(|| self.synth_tys.clone());
                } else {
                    possible_subs
                        .entry(param)
                        .and_modify(|old_tys| {
                            *old_tys = tys.intersection(old_tys).copied().collect()
                        })
                        .or_insert(tys);
                }
            }
        }
        let (possible_subs, maxs): (FxHashMap<_, _>, Vec<_>) = possible_subs
            .into_iter()
            .enumerate()
            .map(|(idx, (gp, tys))| {
                (
                    (gp, (idx, tys.iter().copied().collect::<Vec<_>>())),
                    tys.len(),
                )
            })
            .unzip();
        // print!("Possible subst for {:?}: ", extern_fn.def_id);
        let mut perms = VecIter2::new(maxs);
        let mut possible_perms: Vec<(String, RuslikFnSig<'tcx>)> =
            Vec::with_capacity(perms.total_elems());
        while let Some(perm) = perms.next() {
            let substs: FxHashMap<_, _> = possible_subs
                .iter()
                .map(|(gp, (idx, tys))| (*gp, tys[perm[*idx]]))
                .collect();
            // print!("{:?}, ", substs);
            let substs_name = substs
                .values()
                .map(|ty| crate::suslik_translate::sanitize(&ty.to_string()))
                .intersperse("_".to_string())
                .collect();
            let mut efn = (*extern_fn).clone();
            efn.subst(&mut SubstFolder::from_gens(self.tcx, &substs));
            possible_perms.push((substs_name, efn));
        }
        // println!("");
        possible_perms
    }
}
