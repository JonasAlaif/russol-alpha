use std::collections::hash_map::Entry;

use rustc_ast::Mutability;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def::CtorKind;
use rustc_middle::ty::{
    BoundRegionKind, FreeRegion, GenericArgKind, Region, RegionKind, Ty, TyCtxt, TyKind,
};
use rustc_span::{def_id::DefId, Span};

use crate::{
    hir_translator::{PureFn, PureFnMap},
    ruslik_pure::{BuiltinCallKind, CallInfo, PureExpression, UnOpKind},
    ruslik_types::{self, RuslikFnSig},
    subst_generics::{self, TyFoldable},
    suslik::{
        Assertion, BinOp, BorrowInfo, Clause, Expr, FnSpecKind, Phi, PredArgument, PredMap,
        PredParameter, Predicate, Reason, RustBinOp, SApp, STy, Sigma,
    },
};

pub struct STyTranslator<'a, 'tcx> {
    pub use_full_names: bool,
    pub optimistically_allow_private_types: bool,
    pub tcx: TyCtxt<'tcx>,
    pub map: &'a mut PredMap,
    pub tys: FxHashSet<Ty<'tcx>>,
    pub fn_id: DefId,
}

impl<'a, 'tcx> STyTranslator<'a, 'tcx> {
    pub fn translate_sapp(
        &mut self,
        is_private: bool,
        field_name: &str,
        ty: Ty<'tcx>,
    ) -> Result<SApp, Reason> {
        let ty = self.translate_ty(ty)?;
        Ok(SApp {
            is_private,
            field_name: format!("f{field_name}"),
            ty,
        })
    }

    fn translate_ty(&mut self, ty: Ty<'tcx>) -> Result<STy, Reason> {
        let (is_brrw, inner_ty) = self.ty_to_brrw(ty)?;
        let is_copy = inner_ty
            .is_copy_modulo_regions(self.tcx.at(Span::default()), self.tcx.param_env(self.fn_id));
        let pred = ty_to_pred_name(inner_ty, self.tcx);
        let clean_name = inner_ty.to_string();
        match inner_ty.kind() {
            TyKind::Ref(_, _, _) => unreachable!(),
            TyKind::Bool | TyKind::Int(_) | TyKind::Uint(_) => {
                if let Entry::Vacant(v) = self.map.entry(pred.clone()) {
                    let param = PredParameter::val(FnSpecKind::prim_to_kind(inner_ty));
                    let facts = Expr::prim_to_invs(param.name.clone(), inner_ty.kind());
                    let prim_arg = Some(param.name.clone());
                    v.insert(Predicate {
                        is_prim: true,
                        is_copy,
                        is_drop: false,
                        is_private: false,
                        ident: pred.clone(),
                        clean_name,
                        facts: Phi::empty(), // TODO: put facts here
                        fn_spec: vec![param],
                        clauses: vec![Clause {
                            name: None,
                            prim_arg,
                            selector: true.into(),
                            equalities: FxHashMap::default(),
                            assn: Assertion {
                                phi: facts,
                                sigma: Sigma::empty(),
                            },
                        }],
                    });
                }
                Ok(STy {
                    is_brrw,
                    pred,
                    fn_spec: Vec::new(),
                })
            }
            TyKind::Tuple(t) if t.is_empty() => {
                if let Entry::Vacant(v) = self.map.entry(pred.clone()) {
                    v.insert(Predicate {
                        is_prim: true,
                        is_copy,
                        is_drop: false,
                        is_private: false,
                        ident: pred.clone(),
                        clean_name,
                        facts: Phi::empty(),
                        fn_spec: Vec::new(),
                        clauses: vec![Clause {
                            name: None,
                            prim_arg: None,
                            selector: true.into(),
                            equalities: FxHashMap::default(),
                            assn: Assertion {
                                phi: Phi::empty(),
                                sigma: Sigma::empty(),
                            },
                        }],
                    });
                }
                Ok(STy {
                    is_brrw,
                    pred,
                    fn_spec: Vec::new(),
                })
            }
            TyKind::Adt(adt, subst) => {
                if clean_name.starts_with("core::pin::Pin")
                    || clean_name.starts_with("std::pin::Pin")
                {
                    // Pin is unsupported since it requires the `unsafe_pin_internals` flag
                    return Err(Reason::RequiresFlag);
                }
                let regions = subst.iter().flat_map(|tl_arg| {
                    tl_arg.walk().filter_map(|ga| match ga.unpack() {
                        GenericArgKind::Lifetime(reg) => Some(reg),
                        GenericArgKind::Type(_) => None,
                        GenericArgKind::Const(_) => None,
                    })
                });
                let lft_params = regions
                    .map(|r| {
                        Ok(PredParameter {
                            kind: FnSpecKind::Lft,
                            name: region_to_name(r)?,
                        })
                    })
                    .collect::<Result<Vec<_>, Reason>>()?;

                let lft_args = match self.map.entry(pred.clone()) {
                    Entry::Occupied(e) => {
                        let mut lft_params = lft_params.into_iter();
                        let args = e
                            .get()
                            .fn_spec
                            .iter()
                            .filter(|p| p.kind == FnSpecKind::Lft)
                            .map(|param| PredArgument {
                                name: lft_params.next().unwrap().name,
                                target: param.clone(),
                            })
                            .collect();
                        assert!(lft_params.next().is_none(), "{}", e.get());
                        args
                    }
                    Entry::Vacant(v) => {
                        let mut fn_spec = lft_params.clone();
                        fn_spec.push(PredParameter::default());
                        let mut is_private = !self
                            .tcx
                            .visibility(adt.did())
                            .is_accessible_from(self.fn_id, self.tcx);
                        if self.optimistically_allow_private_types {
                            is_private = false;
                        }
                        let is_drop = adt.has_dtor(self.tcx) && !adt.is_box();
                        if is_private {
                            // private types not currently supported
                            return Err(Reason::PrivateType);
                        }
                        if adt.is_variant_list_non_exhaustive()
                            || adt
                                .variants()
                                .into_iter()
                                .any(|v| v.is_field_list_non_exhaustive())
                        {
                            // non_exhaustive types not currently supported
                            return Err(Reason::NonExhaustive);
                        }
                        // TODO: redo this (temporary workaround for private modules)
                        // the issue is that there may be multiple ways to address a type, e.g.
                        // `some::private::module::Type` or `some::Type` (if `some` reexports `Type`)
                        // it's unclear how to get the correct (non-private) path which `rustc` will accept
                        if clean_name.contains("char_data::tables::BidiClass")
                            || clean_name.contains("error::conversion_range::ConversionRange")
                            || clean_name.contains("proto::peer::Dyn")
                            || clean_name.contains("codec::error::UserError")
                        {
                            return Err(Reason::Other);
                        }
                        v.insert(Predicate {
                            is_prim: false,
                            is_copy,
                            is_drop,
                            is_private,
                            ident: pred.clone(),
                            clean_name: clean_name.clone(),
                            facts: Phi::empty(),
                            fn_spec,
                            clauses: Vec::new(),
                        });
                        let clauses = if let Some(ty) = extract_box_ty(inner_ty) {
                            // println!("TODO: temporary workaround to support boxes");
                            vec![Clause {
                                name: Some("Box::new".to_string()),
                                prim_arg: None,
                                selector: true.into(),
                                equalities: FxHashMap::default(),
                                assn: Assertion {
                                    phi: Phi::empty(),
                                    sigma: Sigma(vec![SApp {
                                        is_private: false,
                                        field_name: "f_666".to_string(),
                                        ty: self.translate_ty(ty)?,
                                    }]),
                                }
                                .add_seq(0.into()),
                            }]
                        } else {
                            adt.variants().iter_enumerated().map(|(vid, v)| {
                                let (dval, name, selector, mut sigma) = if adt.is_enum() {
                                    let disc = adt.discriminant_for_variant(self.tcx, vid);
                                    let mut d = self.translate_sapp(true, "disc", disc.ty)?;
                                    let d_val = d.arg(PredParameter::default(), &None);
                                    assert!(self.map[&d.ty.pred].fn_spec.len() == 1);
                                    // let disc_number = disc.val.into();
                                    let disc_number: Expr = (vid.as_u32() as u128).into();
                                    (disc_number.clone(), Some("::".to_string() + v.name.as_str()), Expr::Var(d_val.name)._eq(disc_number), Sigma(vec![d]))
                                } else {
                                    let name = if v.fields.is_empty() {
                                        let extra = match v.ctor_kind {
                                            CtorKind::Fn => " ()",
                                            CtorKind::Const => "",
                                            CtorKind::Fictive => " {}",
                                        };
                                        Some(extra.to_string())
                                    } else { None };
                                    (0.into(), name, true.into(), Sigma::empty())
                                };
                                let fields = v.fields.iter().map(|fd| {
                                    // println!("{} has fd {:?}", pred, fd);
                                    let is_private = !fd.vis.is_accessible_from(self.fn_id, self.tcx);
                                    let field_name = Self::fd_name_to_sus(vid.as_u32(), fd.name.as_str());
                                    if field_name == PredParameter::default().name {
                                        eprintln!("Fields with name {field_name} clash with internals and will likely cause a crash!");
                                    }
                                    let ty = fd.ty(self.tcx, subst);
                                    self.translate_sapp(is_private, &field_name, ty)
                                }).collect::<Result<Vec<_>, _>>()?;
                                sigma.0.extend(fields);
                                let item_name = if adt.did().is_local() || !self.use_full_names {
                                    if self.use_full_names {
                                        "crate::".to_string() + clean_name.split('<').next().unwrap()
                                    } else {
                                        self.tcx.item_name(adt.did()).as_str().to_string()
                                    }
                                } else {
                                    "::".to_string() + clean_name.split('<').next().unwrap()
                                };
                                let name = Some(item_name + &name.unwrap_or_default());
                                Ok(Clause {
                                    name, prim_arg: None, selector, equalities: FxHashMap::default(),
                                    assn: Assertion { phi: Phi::empty(), sigma }.add_seq(dval),
                                })
                            }).collect::<Result<Vec<_>, Reason>>()?
                        };
                        self.map.get_mut(&pred).unwrap().clauses = clauses;
                        lft_params
                            .into_iter()
                            .map(|p| PredArgument {
                                name: p.name.clone(),
                                target: p,
                            })
                            .collect()
                    }
                };
                Ok(STy {
                    is_brrw,
                    pred,
                    fn_spec: lft_args,
                })
            }
            TyKind::Param(_p) => {
                // let pred = p.name.as_str().to_string();
                if !self.map.contains_key(&pred) {
                    self.map.insert(
                        pred.clone(),
                        Predicate {
                            is_prim: true,
                            is_copy,
                            is_drop: false,
                            is_private: false,
                            ident: pred.clone(),
                            clean_name,
                            facts: Phi::empty(),
                            fn_spec: vec![PredParameter::default()],
                            clauses: Vec::new(),
                        },
                    );
                }
                Ok(STy {
                    is_brrw,
                    pred,
                    fn_spec: Vec::new(),
                })
            }
            TyKind::Tuple(tys) => {
                let regions = tys.iter().flat_map(|tl_arg| {
                    tl_arg.walk().filter_map(|ga| match ga.unpack() {
                        GenericArgKind::Lifetime(reg) => Some(reg),
                        GenericArgKind::Type(_) => None,
                        GenericArgKind::Const(_) => None,
                    })
                });
                let lft_params = regions
                    .map(|r| {
                        Ok(PredParameter {
                            kind: FnSpecKind::Lft,
                            name: region_to_name(r)?,
                        })
                    })
                    .collect::<Result<Vec<_>, Reason>>()?;

                let lft_args = match self.map.entry(pred.clone()) {
                    Entry::Occupied(e) => {
                        let mut lft_params = lft_params.into_iter();
                        let args = e
                            .get()
                            .fn_spec
                            .iter()
                            .filter(|p| p.kind == FnSpecKind::Lft)
                            .map(|param| PredArgument {
                                name: lft_params.next().unwrap().name,
                                target: param.clone(),
                            })
                            .collect();
                        assert!(lft_params.next().is_none(), "{}", e.get());
                        args
                    }
                    Entry::Vacant(v) => {
                        let mut fn_spec = lft_params.clone();
                        fn_spec.push(PredParameter::default());
                        v.insert(Predicate {
                            is_prim: false,
                            is_copy,
                            is_drop: false,
                            is_private: false,
                            ident: pred.clone(),
                            clean_name,
                            facts: Phi::empty(),
                            fn_spec,
                            clauses: Vec::with_capacity(1),
                        });
                        let (selector, mut sigma) = (true.into(), Sigma::empty());
                        let fields = tys
                            .iter()
                            .enumerate()
                            .map(|(idx, ty)| {
                                let is_private = false;
                                let field_name = Self::fd_name_to_sus(0, &idx.to_string());
                                self.translate_sapp(is_private, &field_name, ty)
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        sigma.0.extend(fields);
                        let name = Some("".to_string());
                        let clause = Clause {
                            name,
                            prim_arg: None,
                            selector,
                            equalities: FxHashMap::default(),
                            assn: Assertion {
                                phi: Phi::empty(),
                                sigma,
                            }
                            .add_seq(0.into()),
                        };
                        self.map.get_mut(&pred).unwrap().clauses.push(clause);
                        lft_params
                            .into_iter()
                            .map(|p| PredArgument {
                                name: p.name.clone(),
                                target: p,
                            })
                            .collect()
                    }
                };
                Ok(STy {
                    is_brrw,
                    pred,
                    fn_spec: lft_args,
                })
            }
            TyKind::Char | TyKind::Float(_) => Err(Reason::CharFloat),
            TyKind::Str | TyKind::Array(_, _) | TyKind::Slice(_) => Err(Reason::ArraySlice),
            TyKind::Foreign(_) | TyKind::RawPtr(_) => Err(Reason::Unsafe),
            TyKind::FnDef(_, _)
            | TyKind::FnPtr(_)
            | TyKind::Closure(_, _)
            | TyKind::Generator(_, _, _)
            | TyKind::GeneratorWitness(_) => Err(Reason::Closure),
            TyKind::Dynamic(_, _)
            | TyKind::Never
            | TyKind::Projection(_)
            | TyKind::Opaque(_, _)
            | TyKind::Bound(_, _)
            | TyKind::Placeholder(_)
            | TyKind::Infer(_)
            | TyKind::Error(_) => Err(Reason::OtherTy),
            // _ => Err(format!("Reason ty {}", inner_ty)),
        }
    }

    fn ty_to_brrw(&mut self, mut ty: Ty<'tcx>) -> Result<(Vec<BorrowInfo>, Ty<'tcx>), Reason> {
        self.tys.insert(ty);
        let mut res = Vec::new();
        // let mut first_iter = true;
        while let TyKind::Ref(r, new_ty, m) = *ty.kind() {
            // Only top level
            // if first_iter && !r.is_static() {
            //     blockers.extend(self.possible_blocks.iter().filter(
            //         |(res_lft, _)| self.outlives.free_region_map().sub_free_regions(self.tcx, **res_lft, r)
            //     ).map(|other| other.1.clone()))
            // };
            // first_iter = false;
            res.push(BorrowInfo {
                lft: region_to_name(r)?,
                m,
            });
            ty = new_ty;
            self.tys.insert(ty);
        }
        Ok((res, ty))
    }

    fn fd_name_to_sus(vid: u32, fd: &str) -> String {
        if fd.chars().next().unwrap().is_ascii_digit() {
            format!("{vid}_{fd}")
        } else {
            format!("{vid}{fd}")
        }
    }
}

pub struct ExprTranslator<'tcx, 'a, 'b> {
    pub tcx: TyCtxt<'tcx>,

    pub pre: &'a mut Assertion,
    pub post: &'a mut Assertion,

    pub pure_fns: &'b PureFnMap<'tcx>,
    pub map: &'a mut PredMap,

    pub call_params: Option<(Phi, Vec<PredParameter>)>,
    pub is_fn_body: bool,

    pub under_cond: Vec<(bool, Expr)>,

    pub used_pure_fns: &'a mut Vec<&'b PureFn<'tcx>>,
}

impl<'tcx, 'a, 'b> ExprTranslator<'tcx, 'a, 'b> {
    pub fn translate_expr(
        &mut self,
        expr: &PureExpression<'tcx>,
        mut futs: Vec<bool>,
        param: Option<(PredParameter, Expr)>,
    ) -> Expr {
        // println!("translate_expr: {} ({}) param {:?}", expr, expr.ty(), param);
        match expr.kind() {
            crate::ruslik_pure::ExprKind::Never => todo!(),
            crate::ruslik_pure::ExprKind::Var(v) => {
                let field = format!("f{}", v.rname());
                if let Some((phi, _)) = &self.call_params {
                    assert_eq!(phi.0.len(), 0);
                    // println!("Adding phi: {} to {}; {}", phi, args.phi, args.sigma);
                    // args.phi.0.extend(phi.0.iter().cloned());
                }
                if let Some((param, mut facts)) = param {
                    // println!("Looking for: {}", field);
                    // for app in args.0.iter_mut() {
                    //     println!("field: {} eq {}", app, app.field_name == field);
                    // }
                    let args = if expr.is_result() {
                        &mut self.post.sigma
                    } else {
                        &mut self.pre.sigma
                    };
                    let is_exists = expr.is_result() == (futs.is_empty() || !futs[0]);
                    let phi = if is_exists {
                        &mut self.post.phi
                    } else {
                        &mut self.pre.phi
                    };
                    let app = args.0.iter_mut().find(|app| app.field_name == field);
                    let app = app.expect("Future of reference-typed field not currently supported");

                    let pred = &app.ty.pred.clone();
                    // Only add params if we will actually need them
                    if !self.is_fn_body {
                        let pred = self.map.get_mut(pred).unwrap();
                        if !pred.fn_spec.contains(&param) {
                            pred.fn_spec.push(param.clone());
                        }
                    }
                    let arg = app.arg(param, &self.call_params);
                    let var = if all_futs_current(&futs) {
                        Expr::Var(arg.name)
                    } else {
                        Expr::OnExpiry(futs, arg.kind, field, Err(arg.name))
                    };
                    // Add pure_fn `trusted_ensures`
                    facts.update_result(&var);
                    if !facts.is_true() {
                        phi.0.push(facts);
                    }
                    var
                } else {
                    Expr::Snap(futs, field)
                }
            }
            crate::ruslik_pure::ExprKind::Lit(l) => Expr::Lit(l.into()),
            crate::ruslik_pure::ExprKind::BinOp(op, box l, box r) => {
                assert!(param.is_none());
                assert!(futs.is_empty());
                Expr::BinOp(
                    op.into(),
                    box self.translate_expr(l, Vec::new(), None),
                    box self.translate_expr(r, Vec::new(), None),
                )
            }
            crate::ruslik_pure::ExprKind::UnOp(op, e) => {
                assert!(param.is_none());
                assert!(futs.is_empty());
                match op {
                    // We implicitly take snapshots anyway so no need to do anything
                    UnOpKind::Snap => self.translate_expr(e, futs, None), // otherwise: (e, Some((PredParameter::default(), true.into())), in_post)
                    UnOpKind::UnOp(op) => Expr::UnOp(*op, box self.translate_expr(e, futs, None)),
                }
            }
            crate::ruslik_pure::ExprKind::Constructor(_, _, _) => todo!("{}", expr),
            crate::ruslik_pure::ExprKind::Field(box e, v, f) => {
                if let TyKind::Ref(_, _, m) = e.ty().kind() {
                    let in_post = if (*f, *v) == ruslik_types::OLD {
                        false
                    } else if (*f, *v) == ruslik_types::FUT {
                        true
                    } else {
                        unreachable!()
                    };
                    assert!(
                        !in_post || *m == Mutability::Mut,
                        "It doesn't make sense to `^` immutable references, use `*` instead."
                    );
                    if *m == Mutability::Mut {
                        futs.push(in_post)
                    };
                    return self.translate_expr(e, futs, param);
                }
                // Defaults to structural value (capturing everything) field
                let (mut param, facts) =
                    param.unwrap_or_else(|| (PredParameter::default(), true.into()));
                // Add param to predicate we'll be looking for (need to extract from behind refs first)
                // E.g. for expr `x.f` with `x: Foo` and `x.f: i32` we add param to `i32` pred and will also add arg
                // to `f` field of `Foo` pred (note: the param to `Foo` is not added here yet)
                let mut ty = expr.ty();
                let mut _mut_ref_count = 0;
                while let TyKind::Ref(_, inner_ty, m) = ty.kind() {
                    if *m == Mutability::Mut {
                        _mut_ref_count += 1;
                    }
                    ty = *inner_ty;
                }
                // The `ty` of expr could be behind refs, but should have a corresponding amount of `futs`
                // This can be false when were doing structural eq:
                // assert_eq!(_mut_ref_count as usize, futs.len());
                let pred = ty_to_pred_name(ty.peel_refs(), self.tcx);
                if !self.map.contains_key(&pred) {
                    println!("Could not find pred {} in map {:?}", pred, self.map.keys());
                }
                let pred = self.map.get_mut(&pred).unwrap();
                if ty_is_primitive(ty) {
                    param.kind = FnSpecKind::prim_to_kind(ty)
                };
                // IGNORED for now:
                if let Some((_, call_params)) = &self.call_params {
                    assert!(call_params.is_empty());
                    for call_param in call_params {
                        if !pred.fn_spec.contains(call_param) {
                            pred.fn_spec.push(call_param.clone());
                        }
                    }
                }
                if !pred.fn_spec.contains(&param) {
                    pred.fn_spec.push(param.clone());
                }
                // Now add arg to field in outer predicate (could be a future)
                let fname = ruslik_types::field_to_name(*f, *v, e.ty());
                let vid: usize = v.as_usize();
                // Safety check
                if matches!(e.ty().kind(), TyKind::Adt(_, _) | TyKind::Tuple(_)) {
                } else {
                    todo!("{:?}", expr.ty())
                }
                let pred = ty_to_pred_name(e.ty().peel_refs(), self.tcx);
                if !self.map.contains_key(&pred) {
                    println!("Could not find: {pred} in {:?}", self.map.keys());
                }
                let pred = self.map.get_mut(&pred).unwrap();
                let new_param = if ruslik_types::DISC == (*f, *v) {
                    // Adding arg to `disc` is special since we need to add it in all variants
                    assert_eq!(futs.len(), 0);
                    let mut disc = None;
                    assert_eq!(fname, "disc");
                    let fname = format!("f{}", fname);
                    for clause in &mut pred.clauses {
                        let app = clause
                            .assn
                            .sigma
                            .0
                            .iter_mut()
                            .find(|app| app.field_name == fname)
                            .unwrap();
                        disc = Some(app.arg(param.clone(), &self.call_params));
                    }
                    disc.unwrap()
                } else if extract_box_ty(e.ty()).is_some() {
                    // Special case for box
                    assert_eq!(futs.len(), 0);
                    let arg = pred.clauses[vid].assn.sigma.0[0].arg(param, &self.call_params);
                    to_actual_arg(
                        arg,
                        futs,
                        "f_666".to_string(),
                        &mut pred.clauses[vid].equalities,
                    )
                } else {
                    // General case
                    let fname = STyTranslator::fd_name_to_sus(v.as_u32(), &fname);
                    let fname = format!("f{}", fname);
                    let app = pred.clauses[vid]
                        .assn
                        .sigma
                        .0
                        .iter_mut()
                        .find(|app| app.field_name == fname)
                        .unwrap();
                    let arg = app.arg(param, &self.call_params);
                    to_actual_arg(arg, futs, fname, &mut pred.clauses[vid].equalities)
                };
                self.translate_expr(e, Vec::new(), Some((new_param, facts)))
            }
            crate::ruslik_pure::ExprKind::IfElse(box g, box t, box f) => {
                // Guard
                let call_params = self.call_params.take();
                let g = self.translate_expr(g, Vec::new(), None);
                self.call_params = call_params;
                // True branch
                self.under_cond.push((true, g.clone()));
                let t = self.translate_expr(t, futs.clone(), param.clone());
                // False brach
                self.under_cond.last_mut().unwrap().0 = false;
                let f = self.translate_expr(f, futs, param);
                // Done
                self.under_cond.pop();
                Expr::IfElse(box g, box t, box f)
            }
            crate::ruslik_pure::ExprKind::Call(CallInfo::Pure(id, substs), fn_args) => {
                assert!(
                    ty_is_primitive(expr.ty()),
                    "Return type of fn call ({:?}: {}) must be a primitive.",
                    id,
                    expr.ty()
                );
                assert!(
                    !fn_args.is_empty(),
                    "Fn calls with no arguments are not supported, inline instead."
                );
                assert_eq!(
                    fn_args.len(),
                    1,
                    "Pure functions calls with exactly one argument currently supported ({:?}).",
                    id
                );
                assert!(
                    self.pure_fns.contains_key(&id),
                    "Used a non-pure fn ({id:?}) in specification!"
                );
                let mut pure_fn = self.pure_fns[id].clone();
                let mut substs = subst_generics::SubstFolder::from_substs_ref(self.tcx, substs);
                pure_fn.expr.subst(&mut substs);
                pure_fn.pure_post.subst(&mut substs);
                assert_eq!(pure_fn.arg_names.len(), fn_args.len());
                assert!(param.is_none());
                assert!(futs.is_empty());
                assert!(
                    fn_args.iter().skip(1).all(|arg| ty_is_primitive(arg.ty())),
                    "All additional args must be primitive!"
                );
                let arg_exprs: Vec<_> = fn_args
                    .iter()
                    .skip(1)
                    .map(|arg| self.translate_expr(arg, Vec::new(), None))
                    .collect();
                // Takes immutable ref:
                assert!(fn_args[0].ty().is_ref());
                let target = if let crate::ruslik_pure::ExprKind::Constructor(_, _, fs) =
                    fn_args[0].kind()
                {
                    assert!(fs.len() == 1);
                    &fs[0].1
                } else {
                    unreachable!()
                };

                if let TyKind::Adt(_adt, _) = fn_args[0].ty().peel_refs().kind() {
                    // let pred = self.tcx.item_name(adt.did());
                    // let pred = pred.as_str();
                    let pred = ty_to_pred_name(fn_args[0].ty().peel_refs(), self.tcx);
                    let param_name = self.tcx.item_name(*id).as_str().to_string();
                    assert!(arg_exprs.is_empty());
                    let param_name_result = param_name
                        + "_result"
                        + &arg_exprs
                            .iter()
                            .map(|e| "_".to_string() + &sanitize(&e.to_string()))
                            .collect::<String>();
                    // println!("param_name_result: {}", param_name_result);

                    if !self.map.contains_key(&pred) {
                        println!(
                            "Could not find key {pred} in map {:?} (ty: {})",
                            self.map.keys(),
                            fn_args[0].ty()
                        );
                    }
                    let p = self.map.get_mut(&pred).unwrap();
                    let call_params: Vec<_> = fn_args
                        .iter()
                        .skip(1)
                        .zip(arg_exprs.iter())
                        .map(|(arg, e)| {
                            let param = PredParameter {
                                kind: FnSpecKind::prim_to_kind(arg.ty()),
                                name: sanitize(&e.to_string()),
                            };
                            if !p.fn_spec.contains(&param) {
                                p.fn_spec.push(param.clone())
                            };
                            param
                        })
                        .collect();
                    self.call_params = Some((
                        Phi(call_params
                            .iter()
                            .zip(arg_exprs)
                            .map(|(param, e)| Expr::Var(param.name.clone())._eq(e))
                            .collect()),
                        call_params.clone(),
                    ));
                    // println!("call_params {:?}", call_params);

                    let mut self_app_pre = Sigma(
                        pure_fn
                            .arg_names
                            .iter()
                            .zip(fn_args)
                            .map(|(name, arg)| SApp {
                                is_private: false,
                                field_name: format!("f{}", name.uuid()),
                                ty: STy {
                                    is_brrw: Vec::new(),
                                    pred: ty_to_pred_name(arg.ty().peel_refs(), self.tcx),
                                    fn_spec: Vec::new(),
                                },
                            })
                            .collect(),
                    );
                    for (app, param) in self_app_pre.0.iter_mut().skip(1).zip(call_params.iter()) {
                        assert!(self.map[&app.ty.pred].fn_spec.len() == 1);
                        app.ty.fn_spec.push(PredArgument {
                            name: param.name.clone(),
                            target: self.map[&app.ty.pred].fn_spec[0].clone(),
                        });
                    }
                    let mut res = SApp {
                        is_private: false,
                        field_name: "fresult".to_string(),
                        ty: STy {
                            is_brrw: Vec::new(),
                            pred: "".to_string(),
                            fn_spec: Vec::new(),
                        },
                    };
                    let res_param = res.arg(PredParameter::default(), &None);
                    let mut used_pure_fns = Vec::new();
                    let mut pre = Assertion {
                        phi: Phi::empty(),
                        sigma: self_app_pre,
                    };
                    let mut post = Assertion {
                        phi: Phi::empty(),
                        sigma: Sigma(vec![res]),
                    };
                    let mut translator = ExprTranslator {
                        tcx: self.tcx,
                        pre: &mut pre,
                        post: &mut post,
                        pure_fns: self.pure_fns,
                        map: self.map,
                        call_params: None,
                        is_fn_body: true,
                        under_cond: Vec::new(),
                        used_pure_fns: &mut used_pure_fns,
                    };

                    let mut call_post =
                        translator.translate_expr(&pure_fn.pure_post, Vec::new(), None);
                    // trusted_ensures:
                    if !call_post.is_true() {
                        // TODO: use this instead
                        // call_post.update_result(&Expr::Var(param.name.clone()));
                        // p.facts.0.push(call_post);
                        // TODO: no need for this
                        // Wrap call post under conditionals (otherwise we might have an irrelevant fact in an unrelated enum variant)
                        for &(t, ref cond) in &self.under_cond {
                            call_post = if t {
                                Expr::IfElse(box cond.clone(), box call_post, box true.into())
                            } else {
                                Expr::IfElse(box cond.clone(), box true.into(), box call_post)
                            }
                        }
                    }
                    let param = if let Some(param) = translator.map[&pred]
                        .fn_spec
                        .iter()
                        .find(|param| param.name == param_name_result)
                    {
                        param.clone()
                    } else {
                        self.used_pure_fns.push(&self.pure_fns[id]);
                        let param = PredParameter {
                            kind: FnSpecKind::prim_to_kind(expr.ty()),
                            name: param_name_result.clone(),
                        };
                        let p = translator.map.get_mut(&pred).unwrap();
                        // println!("Adding {} to {}", param, p);
                        p.fn_spec.push(param.clone());

                        // println!("Translating: {}", fn_body);
                        let mut body = translator.translate_expr(&pure_fn.expr, Vec::new(), None);
                        // Remove "_self_old"
                        let param_names = call_params.iter().map(|param| &param.name);
                        let mut arg_map: FxHashMap<_, _> = call_params
                            .iter()
                            .map(|n| n.name.as_str())
                            .zip(param_names)
                            .collect();
                        let pnr = param_name_result.clone();
                        arg_map.insert(res_param.name.as_str(), &pnr);
                        let self_name = format!("f{}", pure_fn.arg_names[0].uuid());
                        let f = |v: &mut String| {
                            *v = if arg_map.contains_key(v.as_str()) {
                                arg_map[v.as_str()].clone()
                            } else {
                                // let v = &v[..v.rfind('_').unwrap()];
                                assert_eq!(&v[v.rfind('_').unwrap() + 1..], self_name);
                                v[..v.rfind('_').unwrap()].to_string()
                            }
                        };
                        body.update_vars(&f);
                        // TODO: may also want to use `translator.post.phi.0`?
                        for e in &mut translator.pre.phi.0 {
                            e.update_vars(&f);
                        }
                        for Clause {
                            equalities, assn, ..
                        } in translator.map.get_mut(&pred).unwrap().clauses.iter_mut()
                        {
                            equalities.insert(param_name_result.clone(), body.clone());
                            assn.phi.0.extend(translator.pre.phi.0.iter().cloned());
                        }
                        self.used_pure_fns.extend(used_pure_fns);
                        param
                    };
                    // println!("Call with param: {}", param);
                    let call_expr =
                        self.translate_expr(target, Vec::new(), Some((param, call_post)));
                    // println!("Done with call");
                    self.call_params = None;
                    call_expr
                } else {
                    todo!("{}", expr)
                }
            }
            crate::ruslik_pure::ExprKind::Call(
                CallInfo::Builtin(BuiltinCallKind::SetConstruct),
                args,
            ) => {
                if let crate::ruslik_pure::ExprKind::Constructor(_, _, args) =
                    args.first().unwrap().kind()
                {
                    Expr::Tuple(
                        true,
                        args.iter()
                            .map(|arg| {
                                if let crate::ruslik_pure::ExprKind::Constructor(_, _, args) =
                                    arg.1.kind()
                                {
                                    self.translate_expr(&args.first().unwrap().1, Vec::new(), None)
                                } else {
                                    todo!("Array element isn't a reference!")
                                }
                            })
                            .collect(),
                    )
                } else {
                    todo!("Sets can only be constructed directly with arrays!")
                }
            }
            crate::ruslik_pure::ExprKind::Call(
                CallInfo::Builtin(BuiltinCallKind::SetContains),
                args,
            ) => {
                assert!(param.is_none());
                let set = self.translate_expr(&args[0], Vec::new(), None);
                let elem = self.translate_expr(&args[1], Vec::new(), None);
                Expr::BinOp(BinOp::SetContains, box elem, box set)
            }
        }
    }

    #[allow(dead_code)]
    fn translate_borrow_info(bi: Option<&BorrowInfo>, blockers: &[String]) -> Expr {
        blockers.iter().fold(
            true.into(),
            // Lifetime name manipulation here:
            |acc, blocker| {
                acc & Expr::BinOp(
                    RustBinOp::Ge.into(),
                    box Expr::Var(format!("&{}", &bi.unwrap().lft[1..])),
                    box Expr::Var(format!("&{}", &blocker[1..])),
                )
            },
        )
    }
}

fn to_actual_arg(
    arg: PredParameter,
    futs: Vec<bool>,
    fname: String,
    equalities: &mut FxHashMap<String, Expr>,
) -> PredParameter {
    if !arg.kind.is_snap() && all_futs_current(&futs) {
        return arg;
    }
    let futs_str = futs
        .iter()
        .map(|f| if *f { "f" } else { "c" })
        .collect::<String>();
    let (new_name, expr) = if arg.kind.is_snap() {
        let new_name = arg.name.clone() + "_snap" + &futs_str;
        (new_name, Expr::Snap(futs, fname))
    } else if !all_futs_current(&futs) {
        let new_name = arg.name.clone() + "_" + &futs_str;
        (
            new_name,
            Expr::OnExpiry(futs, arg.kind, fname, Err(arg.name)),
        )
    } else {
        panic!()
    };
    equalities.insert(new_name.clone(), expr);
    PredParameter {
        kind: if arg.kind.is_snap() {
            FnSpecKind::Int
        } else {
            arg.kind
        },
        name: new_name,
    }
}

fn region_to_name(r: Region) -> Result<String, Reason> {
    match r.kind() {
        RegionKind::ReEarlyBound(ebr) => Ok(ebr.name.to_string()),
        // late bound regions unsupported
        // Example: type FormatFn = Box<dyn Fn(&mut Formatter, &Record) -> io::Result<()> + Sync + Send>;
        RegionKind::ReLateBound(_, _) => Err(Reason::LateBoundRegion),
        RegionKind::ReFree(FreeRegion {
            bound_region: BoundRegionKind::BrAnon(i),
            ..
        }) => Ok(format!("'anon{}", i)),
        RegionKind::ReFree(FreeRegion {
            bound_region: BoundRegionKind::BrNamed(_, n),
            ..
        }) => Ok(n.to_string()),
        RegionKind::ReFree(FreeRegion {
            bound_region: BoundRegionKind::BrEnv,
            ..
        }) => todo!(),
        RegionKind::ReStatic => Ok("'static".to_string()),
        RegionKind::ReVar(_) => todo!(),
        RegionKind::RePlaceholder(_) => todo!(),
        RegionKind::ReErased => todo!(),
    }
}

// include_refs: should only be true for generics
fn ty_to_pred_name(mut ty: Ty, tcx: TyCtxt) -> String {
    let mut prefix = String::new();
    while let TyKind::Ref(_, inner_ty, m) = ty.kind() {
        if *m == Mutability::Mut {
            prefix += "Rmut"
        } else {
            prefix += "R"
        };
        ty = *inner_ty;
    }
    let pred = match ty.kind() {
        TyKind::Adt(adt, subst) => {
            let cid = adt.did().index.as_u32();
            let pred = tcx.item_name(adt.did());
            let pred = pred.as_str().to_string();
            let ty_params = subst
                .types()
                .map(|ty| ty_to_pred_name(ty, tcx))
                .fold(String::new(), |acc, p| acc + "_" + &p);
            prefix + &cid.to_string() + "_" + &pred + &ty_params + "_"
        }
        _ => prefix + &sanitize(&ty.to_string()),
    };
    // Workaround to avoid suslik "bool" keyword
    // pred.get_mut(0..1).unwrap().make_ascii_uppercase();
    "P".to_string() + &pred
}

pub fn ty_is_primitive(ty: Ty) -> bool {
    ty.is_primitive()
        || matches!(ty.to_string().as_str(), ty if ty.starts_with("russol_contracts::Set"))
}

fn extract_box_ty<'tcx>(ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    if ty.is_box() {
        Some(ty.boxed_ty())
    } else {
        None
    }
}

pub(crate) fn all_futs_current(futs: &[bool]) -> bool {
    // futs.len() == 0
    futs.iter().all(|fut| !fut)
}

pub(crate) fn outlives_relations<'tcx>(
    tcx: TyCtxt<'tcx>,
    sig: &RuslikFnSig<'tcx>,
) -> Vec<(String, String)> {
    // let (sources, dests) = fnsig_regions::collect_blocking_lfts(sig.args.iter().map(|(_, ty)| *ty).collect(), sig.ret, tcx);
    let mut rels = Vec::new();
    for left in sig.outlives.free_region_map().elements() {
        for right in sig.outlives.free_region_map().elements() {
            if left == right {
                continue;
            }
            // println!("Checking {left} vs {right}: {}", sig.outlives.free_region_map().sub_free_regions(tcx, *left, *right));
            if sig
                .outlives
                .free_region_map()
                .sub_free_regions(tcx, left, right)
            {
                rels.push((
                    region_to_name(left).unwrap(),
                    region_to_name(right).unwrap(),
                ))
            }
        }
    }
    rels
}

pub fn sanitize(s: &str) -> String {
    s.chars()
        .filter_map(|c| match c {
            ':' => Some('_'),
            '<' => Some('_'),
            '>' => None,
            ' ' => Some('_'),
            '&' => None,
            ',' => Some('_'),
            // Raw pointer, e.g.: *mut
            '*' => None,
            // Fn e.g.: fn(T) -> S
            '-' => Some('_'),
            '\'' => None,
            // impl AsRef<[u8]>
            '[' => Some('_'),
            ']' => Some('_'),
            '(' => None,
            ')' => None,
            // r#dyn
            '#' => Some('_'),
            // impl Trait + 'static
            '+' => None,
            // Trait<Item = T>
            '=' => Some('_'),
            _ => Some(c),
        })
        .collect()
}
