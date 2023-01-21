use std::fmt;

use rustc_ast::Mutability;
use rustc_data_structures::fx::FxHashMap;

use crate::{
    subst_generics::VecIter,
    suslik::{
        Assertion, BinOp, Clause, Expr, FnSpecKind, Lit, Phi, PredArgument, PredParameter,
        Predicate, SApp, Sigma, Signature, SuslikProgram, UnOp,
    },
    suslik_translate::all_futs_current,
};

type PredEnv = FxHashMap<String, (Phi, Vec<PredParameter>)>;

impl Assertion {
    pub(crate) fn add_seq(mut self, dval: Expr) -> Self {
        self.phi.0.push(
            Expr::Var(PredParameter::default().name)._eq(Expr::Tuple(
                false,
                std::iter::once(dval)
                    .chain(
                        self.sigma
                            .0
                            .iter()
                            .map(|sapp| Expr::Snap(Vec::new(), sapp.field_name.clone())),
                    )
                    .collect(),
            )),
        );
        self
    }
}

impl SApp {
    pub(crate) fn normalize(&mut self, pred_sigs: &PredEnv) {
        for fn_spec in &self.ty.fn_spec {
            assert!(
                pred_sigs[&self.ty.pred]
                    .1
                    .iter()
                    .any(|param| param.name == fn_spec.target.name),
                "Could not find {}->{} in {:?}",
                fn_spec,
                fn_spec.target,
                pred_sigs[&self.ty.pred]
            );
        }
        self.ty.fn_spec = pred_sigs[&self.ty.pred]
            .1
            .iter()
            .map(|param| {
                let pos = self.ty.fn_spec.iter().position(|arg| &arg.target == param);
                pos.map(|i| self.ty.fn_spec.swap_remove(i))
                    .unwrap_or_else(|| {
                        let new_name = param.name.clone() + "_" + &self.field_name;
                        PredArgument {
                            name: new_name,
                            target: param.clone(),
                        }
                    })
            })
            .collect();
    }

    pub(crate) fn arg(
        &mut self,
        param: PredParameter,
        call_params: &Option<(Phi, Vec<PredParameter>)>,
    ) -> PredParameter {
        // println!("Getting arg of {} p: {}", self, param);
        if let Some((_, call_params)) = call_params {
            assert!(call_params.is_empty());
            for call_param in call_params {
                if !self
                    .ty
                    .fn_spec
                    .iter()
                    .any(|arg| arg.target.name == call_param.name)
                {
                    self.ty.fn_spec.push(PredArgument {
                        name: call_param.name.clone(),
                        target: call_param.clone(),
                    });
                }
            }
        }
        if let Some(arg) = self
            .ty
            .fn_spec
            .iter()
            .find(|arg| arg.target.name == param.name)
        {
            PredParameter {
                kind: param.kind,
                name: arg.name.clone(),
            }
        } else {
            let arg_name = param.name.clone() + "_" + &self.field_name;
            let kind = param.kind;
            self.ty.fn_spec.push(PredArgument {
                name: arg_name.clone(),
                target: param,
            });
            PredParameter {
                kind,
                name: arg_name,
            }
        }
    }
}
impl Sigma {
    pub(crate) fn normalize(
        &mut self,
        _phi: &mut Phi,
        mut _other: Option<&mut Phi>,
        pred_sigs: &PredEnv,
    ) {
        for sapp in &mut self.0 {
            sapp.normalize(pred_sigs);

            // // TODO: implement this properly
            // let missing_futs = sapp.ty.is_brrw.iter().filter(|b| b.m == Mutability::Mut).count();
            // let mut perms = VecIter::new(0, 2, missing_futs);
            // let mut possible_futs = Vec::with_capacity(perms.total_elems());
            // while let Some(futs_fill) = perms.next() {
            //     possible_futs.push(futs_fill.iter().map(|i| if *i == 0 { false } else { true }).collect::<Vec<_>>());
            // }

            // let facts: &Phi = &pred_sigs[&sapp.ty.pred].0;
            // for pf in possible_futs {
            //     let phi = if *pf.first().unwrap_or(&false) {
            //         if let Some(other) = &mut other { &mut **other }
            //         else {
            //             panic!("`trusted_ensures` on reference typed field currently not supported")
            //         }
            //     } else { &mut *phi };
            //     phi.0.extend(facts.0.clone().into_iter().map(|mut e| {
            //         e.change_var(&|v|
            //             sapp.ty.fn_spec.iter().find(|arg| arg.target.name == *v).map(|arg| {
            //                 if all_futs_current(&pf) {
            //                     Expr::Var(arg.name.clone())
            //                 } else {
            //                     Expr::OnExpiry(pf.clone(), arg.target.kind, sapp.field_name.clone(), Err(arg.name.clone()))
            //                 }
            //             })
            //         );
            //         e
            //     }));
            // }
        }
    }
}
impl Predicate {
    pub(crate) fn normalize(&mut self, pred_sigs: &PredEnv) {
        for clause in &mut self.clauses {
            clause
                .assn
                .sigma
                .normalize(&mut clause.assn.phi, None, pred_sigs);
            let patcher = SnapPatcher {
                pre: &clause.assn.sigma,
                post: &Sigma::empty(),
            };
            for pre in &mut clause.assn.phi.0 {
                patcher.patch_snap(pre);
            }
            for pre in clause.equalities.values_mut() {
                patcher.patch_snap(pre);
            }
        }
    }
}
impl Signature {
    pub(crate) fn normalize(&mut self, pred_sigs: &PredEnv) {
        let trues: Vec<_> = self
            .pre
            .phi
            .0
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(i, e)| if e.is_true() { Some(i) } else { None })
            .collect();
        for i in trues {
            self.pre.phi.0.swap_remove(i);
        }
        self.pre
            .sigma
            .normalize(&mut self.pre.phi, Some(&mut self.post.phi), pred_sigs);
        let trues: Vec<_> = self
            .post
            .phi
            .0
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(i, e)| if e.is_true() { Some(i) } else { None })
            .collect();
        for i in trues {
            self.post.phi.0.swap_remove(i);
        }
        assert!(self.post.sigma.0.len() <= 1);
        assert!(self
            .post
            .sigma
            .0
            .first()
            .map(|s| s.field_name == "fresult")
            .unwrap_or(true));
        self.post
            .sigma
            .normalize(&mut self.post.phi, Some(&mut self.pre.phi), pred_sigs);
        let patcher = SnapPatcher {
            pre: &self.pre.sigma,
            post: &self.post.sigma,
        };
        for pre in &mut self.pre.phi.0 {
            patcher.patch_snap(pre);
        }
        for post in &mut self.post.phi.0 {
            patcher.patch_snap(post);
        }
    }
}

impl SuslikProgram {
    pub(crate) fn normalize(&mut self) {
        let pred_sigs: FxHashMap<_, _> = self
            .pred_map
            .iter()
            .map(|(name, pred)| (name.clone(), (pred.facts.clone(), pred.fn_spec.clone())))
            .collect();
        for pred in self.pred_map.values_mut() {
            pred.normalize(&pred_sigs);
        }
        self.synth_fn.normalize(&pred_sigs);
        for efn in &mut self.extern_fns {
            efn.normalize(&pred_sigs);
        }
    }
}

struct SnapPatcher<'a> {
    pre: &'a Sigma,
    post: &'a Sigma,
}
impl<'a> SnapPatcher<'a> {
    pub fn patch_snap(&self, e: &mut Expr) {
        match e {
            Expr::Var(_) => (),
            Expr::Snap(futs, f) => {
                let sig = if f == "fresult" { self.post } else { self.pre };
                let app = sig.0.iter().find(|app| app.field_name == *f);
                if app.is_none() {
                    println!("Could not find field `{}` in sig `{}`!", f, sig);
                }
                let missing_futs = app
                    .unwrap()
                    .ty
                    .is_brrw
                    .iter()
                    .filter(|b| b.m == Mutability::Mut)
                    .count()
                    - futs.len();
                let fn_spec = app
                    .unwrap()
                    .ty
                    .fn_spec
                    .iter()
                    .enumerate()
                    .filter(|(_, arg)| arg.target.kind != FnSpecKind::Lft);
                let mut perms = VecIter::new(0, 2, missing_futs);
                let mut possible_futs = Vec::with_capacity(perms.total_elems());
                // TODO: refactor
                while let Some(futs_fill) = perms.next() {
                    futs.extend(futs_fill.iter().map(|i| *i != 0));
                    possible_futs.push(Expr::Tuple(
                        false,
                        fn_spec
                            .clone()
                            .map(|(idx, arg)| {
                                if all_futs_current(futs) {
                                    Expr::Var(arg.name.clone())
                                } else {
                                    Expr::OnExpiry(
                                        futs.clone(),
                                        arg.target.kind,
                                        f.clone(),
                                        Ok(idx),
                                    )
                                }
                            })
                            .collect(),
                    ));
                    futs.truncate(futs.len() - futs_fill.len());
                }
                *e = if missing_futs == 0 {
                    possible_futs.swap_remove(0)
                } else {
                    Expr::Tuple(false, possible_futs)
                };
            }
            Expr::OnExpiry(futs, _, f, Err(arg)) => {
                assert!(!all_futs_current(futs));
                let sig = if f == "fresult" {
                    &self.post
                } else {
                    &self.pre
                };
                let app = sig.0.iter().find(|app| app.field_name == *f).unwrap();
                let idx = app.ty.fn_spec.iter().position(|a| a.name == *arg).unwrap();
                if let Expr::OnExpiry(_, _, _, i) = e {
                    *i = Ok(idx);
                } else {
                    unreachable!()
                }
            }
            Expr::OnExpiry(..) => unreachable!("{}", e),
            Expr::Tuple(_, es) => {
                for e in es {
                    self.patch_snap(e)
                }
            }
            Expr::Lit(_) => (),
            Expr::BinOp(_, l, r) => {
                self.patch_snap(l);
                self.patch_snap(r);
            }
            Expr::UnOp(_, e) => self.patch_snap(e),
            Expr::IfElse(c, t, f) => {
                self.patch_snap(c);
                self.patch_snap(t);
                self.patch_snap(f);
            }
        }
    }
}

impl fmt::Display for SuslikProgram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for pred in self.pred_map.values() {
            writeln!(f, "{}", pred)?;
        }
        for efn in &self.extern_fns {
            writeln!(f, "{}", efn)?;
        }
        writeln!(f, "{}", self.synth_fn)
    }
}
impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{\n  ")?;
        if !self.region_rels.is_empty() {
            // Lifetime name manipulation here:
            write!(
                f,
                "&{} <= &{} ",
                &self.region_rels[0].0[1..],
                &self.region_rels[0].1[1..]
            )?;
            for (left, right) in self.region_rels[1..].iter() {
                // Lifetime name manipulation here:
                write!(f, "&& &{} <= &{} ", &left[1..], &right[1..])?;
            }
            if self.pre.phi.0.is_empty() {
                write!(f, ";\n  ")?;
            } else {
                write!(f, "&&\n  ")?;
            }
        }
        writeln!(f, "{} {}\n}}", self.pre.phi, self.pre.sigma)?;
        writeln!(f, "{} \"{}\"", self.unique_name, self.fn_name)?;
        writeln!(f, "{{\n  {} {}\n}}", self.post.phi, self.post.sigma)
    }
}
impl fmt::Display for Phi {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "")?;
        } else {
            write!(f, "{}", self.0[0])?;
            for expr in self.0[1..].iter() {
                write!(f, " &&\n  {}", expr)?;
            }
            write!(f, " ;\n  ")?;
        }
        Ok(())
    }
}
impl fmt::Display for Sigma {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "emp")?;
        } else {
            write!(f, "{}", self.0[0])?;
            for app in self.0[1..].iter() {
                write!(f, " **\n   {}", app)?;
            }
        }
        Ok(())
    }
}
impl fmt::Display for SApp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_private {
            write!(f, "priv ")?;
        }
        write!(f, "{}: ", self.field_name)?;
        for lft in &self.ty.is_brrw {
            write!(f, "&{} ", &lft.lft[1..])?;
            if matches!(lft.m, rustc_hir::Mutability::Mut) {
                write!(f, "mut ")?;
            }
        }
        // if self.ty.is_opaque { write!(f, "OPQ_")?; }
        write!(f, "{}(", self.ty.pred)?;
        if !self.ty.fn_spec.is_empty() {
            write!(f, "{}", self.ty.fn_spec[0])?;
            for arg in &self.ty.fn_spec[1..] {
                write!(f, ", {}", arg)?;
            }
        }
        write!(f, ")")?;
        // if self.ty.blockers.len() >= 1 {
        //     let b_str = self.ty.blockers.iter().map(|s| &s[1..]).intersperse(" + ").collect::<String>();
        //     write!(f, "<{}>", b_str)?;
        // }
        Ok(())
    }
}
impl fmt::Display for PredArgument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Lifetime name manipulation here:
        if self.target.kind == FnSpecKind::Lft {
            write!(f, "&{}", &self.name[1..])
        } else {
            write!(f, "{}", self.name)
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Var(v) => write!(f, "{}", v),
            Expr::Snap(futs, fd) => write!(f, "snap({:?}, {})", futs, fd),
            Expr::OnExpiry(futs, ty, field, Ok(idx)) => write!(
                f,
                "{}({})[{}]",
                futs.iter()
                    .map(|fut| if *fut { "^ " } else { "* " })
                    .collect::<String>(),
                PredParameter {
                    kind: *ty,
                    name: field.clone()
                },
                idx
            ),
            Expr::OnExpiry(futs, ty, field, Err(arg)) => write!(
                f,
                "{}({})<:{}:>",
                futs.iter()
                    .map(|fut| if *fut { "^" } else { "*" })
                    .collect::<String>(),
                PredParameter {
                    kind: *ty,
                    name: field.clone()
                },
                arg
            ),
            Expr::Tuple(true, es) => write!(
                f,
                "{{{}}}",
                es.iter()
                    .map(|e| e.to_string())
                    .intersperse(", ".to_string())
                    .collect::<String>()
            ),
            Expr::Tuple(false, es) => write!(
                f,
                "({})",
                es.iter()
                    .map(|e| e.to_string())
                    .intersperse(", ".to_string())
                    .collect::<String>()
            ),
            // We don't care about `LitIntType` since we know the exact type anyway
            Expr::Lit(Lit::Int(i, _)) => write!(f, "{}", i),
            Expr::Lit(Lit::Bool(b)) => write!(f, "{}", b),
            Expr::Lit(l) => panic!("Unsupported lit {:?}", l),
            Expr::BinOp(BinOp::Rust(op), box l, box r) => {
                write!(f, "({} {} {})", l, op.as_str(), r)
            }
            Expr::BinOp(BinOp::SetContains, box l, box r) => write!(f, "({} in {})", l, r),
            Expr::UnOp(UnOp::Not, box e) => write!(f, "(not {})", e),
            Expr::UnOp(UnOp::Neg, box e) => write!(f, "(- {})", e),
            Expr::IfElse(box g, box t, box e) => write!(f, "({} ? {} : {})", g, t, e),
        }
    }
}
impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_private {
            write!(f, "priv ")?;
        }
        write!(f, "predicate ")?;
        if self.is_prim && !self.clauses.is_empty() {
            write!(f, "PRIM_")?;
        }
        write!(f, "{}", self.ident)?;
        assert!(!(self.is_copy && self.is_drop));
        if self.is_copy {
            write!(f, "_COPY")?;
        }
        if self.is_drop {
            write!(f, "_DROP")?;
        }
        write!(f, "(")?;
        if !self.fn_spec.is_empty() {
            write!(f, "{}", self.fn_spec[0])?;
            for param in &self.fn_spec[1..] {
                write!(f, ", {}", param)?;
            }
        }
        writeln!(f, ") \"{}\" {{", self.clean_name)?;
        for clause in &self.clauses {
            writeln!(f, "{}", clause)?;
        }
        writeln!(f, "}}")
    }
}
impl fmt::Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pure = self
            .equalities
            .iter()
            .map(|(var, e)| Expr::Var(var.clone())._eq(e.clone()));
        let mut pure: Vec<_> = pure.chain(self.assn.phi.0.clone()).collect();
        if let Some(prim_arg) = &self.prim_arg {
            pure.push(Expr::Var(format!("#[{}]", prim_arg)));
        }
        let phi = Phi(pure);
        write!(f, "| {} => ", self.selector)?;
        if let Some(name) = &self.name {
            write!(f, "\"{}\" ", name)?;
        }
        write!(f, "{{\n  {} {}\n }}", phi, self.assn.sigma)
    }
}
impl fmt::Display for PredParameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            FnSpecKind::Int => write!(f, "int {}", self.name),
            FnSpecKind::Snap => write!(f, "int {}", self.name),
            FnSpecKind::Bool => write!(f, "bool {}", self.name),
            // FnSpecKind::Tpl => write!(f, "tpl {}", self.name),
            // Lifetime name manipulation here:
            FnSpecKind::Lft => write!(f, "lft &{}", &self.name[1..]),
            FnSpecKind::Set => write!(f, "set {}", self.name),
        }
    }
}
