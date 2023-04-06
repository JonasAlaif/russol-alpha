use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use rustc_ast::LitIntType;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def::DefKind;
use rustc_middle::ty::{Ty, TyCtxt, TyKind};
use rustc_type_ir::{IntTy, UintTy};
use wait_timeout::ChildExt;

use crate::{
    hir_translator::{PureFn, PureFnMap},
    ruslik_types::RuslikFnSig,
    subst_generics::SGenericsCollector,
    suslik_translate::{outlives_relations, ExprTranslator, STyTranslator},
    trait_bounds::find_trait_fns,
};

pub type PredMap = FxHashMap<String, Predicate>;
#[derive(Debug, Copy, Clone)]
pub enum BinOp {
    Rust(RustBinOp),
    SetContains,
}
impl From<RustBinOp> for BinOp {
    fn from(rust: RustBinOp) -> Self {
        BinOp::Rust(rust)
    }
}
impl From<&RustBinOp> for BinOp {
    fn from(rust: &RustBinOp) -> Self {
        BinOp::Rust(*rust)
    }
}
pub type RustBinOp = rustc_hir::BinOpKind;
pub type UnOp = rustc_middle::mir::UnOp;
#[derive(Debug, Clone)]
pub enum Lit {
    Int(u128, rustc_ast::ast::LitIntType),
    Bool(bool),
    Unsupported(String),
}
impl From<&rustc_ast::ast::LitKind> for Lit {
    fn from(lit: &rustc_ast::ast::LitKind) -> Self {
        match lit {
            &rustc_ast::ast::LitKind::Int(i, t) => Lit::Int(i, t),
            &rustc_ast::ast::LitKind::Bool(b) => Lit::Bool(b),
            other => Lit::Unsupported(format!("{other:?}")),
        }
    }
}
// pub type Lit = rustc_ast::ast::LitKind;

pub struct SuslikProgram {
    pub(crate) pred_map: PredMap,
    pub(crate) extern_fns: Vec<Signature>,
    pub(crate) synth_fn: Signature,
    pub(crate) synth_ast: usize,
    pub(crate) pure_fn_ast: UsedPureFns,
}

pub struct Signature {
    pub(crate) is_trivial: bool,
    pub(crate) region_rels: Vec<(String, String)>,
    pub(crate) pre: Assertion,
    pub(crate) post: Assertion,
    pub(crate) unique_name: String,
    pub(crate) fn_name: String,
}

pub struct Predicate {
    pub is_prim: bool,
    pub is_copy: bool,
    pub is_drop: bool,
    pub is_private: bool,
    pub ident: String, // used as key for Predicate map
    pub clean_name: String,
    // Fill in from fn_spec
    pub facts: Phi,
    pub fn_spec: Vec<PredParameter>,
    pub clauses: Vec<Clause>,
}
pub struct Clause {
    pub name: Option<String>,
    pub prim_arg: Option<String>,
    pub selector: Expr,
    // Used as `Var(key) == value` for fns and futures
    pub equalities: FxHashMap<String, Expr>,
    pub assn: Assertion,
}

#[derive(Clone)]
pub struct SApp {
    pub is_private: bool,
    pub field_name: String,
    pub ty: STy,
}
#[derive(Clone)]
pub struct STy {
    // pub is_opaque: bool,
    pub is_brrw: Vec<BorrowInfo>,
    pub pred: String, // used as index into Predicate map
    pub fn_spec: Vec<PredArgument>,
}
#[derive(Clone, Debug)]
pub struct BorrowInfo {
    pub lft: String,
    pub m: rustc_hir::Mutability,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FnSpecKind {
    Int,
    Bool,
    Lft,
    Set,
    Snap, //, Tpl
}

impl FnSpecKind {
    pub fn prim_to_kind<'tcx>(ty: Ty<'tcx>) -> Self {
        match ty.kind() {
            TyKind::Bool => Self::Bool,
            TyKind::Char => todo!(),
            TyKind::Int(_) | TyKind::Uint(_) => Self::Int,
            TyKind::Float(_) => todo!(),
            TyKind::Tuple(t) if t.is_empty() => Self::Int,
            TyKind::Adt(_, _) => match ty.to_string().as_str() {
                ty if ty.starts_with("russol_contracts::Set") => Self::Set,
                _ => todo!(),
            },
            _ => unreachable!(),
        }
    }
    pub fn is_snap(&self) -> bool {
        matches!(self, FnSpecKind::Snap)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PredParameter {
    pub kind: FnSpecKind,
    pub name: String,
}
impl Default for PredParameter {
    fn default() -> Self {
        Self {
            kind: FnSpecKind::Snap,
            name: "snap".to_string(),
        }
    }
}
impl PredParameter {
    pub fn val(kind: FnSpecKind) -> Self {
        Self {
            kind,
            name: "snap".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PredArgument {
    pub name: String,
    pub target: PredParameter,
}

#[derive(Debug, Clone)]
pub enum Expr {
    // Never,
    Var(String),
    // Futures deref of snapshot
    Snap(Vec<bool>, String),
    // Futures, ty, field?
    OnExpiry(Vec<bool>, FnSpecKind, String, Result<usize, String>),
    // true -> is set, false -> is tuple
    Tuple(bool, Vec<Expr>),
    Lit(Lit),
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    UnOp(UnOp, Box<Expr>),
    IfElse(Box<Expr>, Box<Expr>, Box<Expr>),
}

pub struct Assertion {
    pub phi: Phi,
    pub sigma: Sigma,
}
impl Assertion {
    pub fn empty() -> Self {
        Self {
            phi: Phi::empty(),
            sigma: Sigma::empty(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Phi(pub Vec<Expr>);
impl Phi {
    pub fn empty() -> Self {
        Self(Vec::new())
    }
}
pub struct Sigma(pub Vec<SApp>);
impl Sigma {
    pub fn empty() -> Self {
        Self(Vec::new())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Unsupported {
    pub in_main: bool,
    pub reason: Reason,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum Reason {
    UnnamedArgs,
    LateBoundRegion,
    RequiresFlag,
    PrivateType,
    NonExhaustive,
    Other,
    CharFloat,
    ArraySlice,
    Closure,
    Unsafe,
    OtherTy,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SynthesisResult {
    pub is_trivial: bool,
    pub kind: SynthesisResultKind,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum SynthesisResultKind {
    Unsupported(Unsupported),
    Unsolvable(u64),
    Timeout,
    Solved(Solved),
}
impl SynthesisResult {
    pub fn get_solved(&self) -> Option<&Solved> {
        if let SynthesisResultKind::Solved(sln) = &self.kind {
            Some(sln)
        } else {
            None
        }
    }
    pub fn get_unsupported(&self) -> Option<&Unsupported> {
        if let SynthesisResultKind::Unsupported(u) = &self.kind {
            Some(u)
        } else {
            None
        }
    }
    pub fn get_unsolvable(&self) -> Option<u64> {
        if let SynthesisResultKind::Unsolvable(u) = &self.kind {
            Some(*u)
        } else {
            None
        }
    }
    pub fn is_timeout(&self) -> bool {
        matches!(self.kind, SynthesisResultKind::Timeout)
    }
}
pub type UsedPureFns = FxHashMap<String, (bool, usize)>;
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Solved {
    pub exec_time: u64,
    pub synth_ast: usize,
    pub pure_fn_ast: UsedPureFns,
    pub slns: Vec<Solution>,
}
impl Solved {
    fn new(exec_time: u64, synth_ast: usize, pure_fn_ast: UsedPureFns, sln: String) -> Self {
        let idx = &mut 0;
        let slns: Vec<_> = sln
            .split("-----------------------------------------------------\n")
            .flat_map(|sln| Solution::new(sln, idx))
            .collect();
        Self {
            exec_time,
            synth_ast,
            pure_fn_ast,
            slns,
        }
    }
    pub fn print(&self) {
        let min_lines_print = std::env::var("RUSLIC_PRINT_SLN_ABOVE")
            .map(|v| v.parse::<usize>().unwrap())
            .unwrap_or(0);
        for sln in &self.slns {
            if sln.loc > min_lines_print {
                if self.slns.len() > 1 {
                    println!("[Solution #{}]", sln.idx);
                }
                print!("{}", sln.code);
            }
        }
    }
}
pub struct MeanStats {
    pub synth_time: f64,
    pub loc: f64,
    pub spec_ast: f64,
    pub ast_nodes: f64,
    pub ast_nodes_unsimp: f64,
    pub rule_apps: f64,
}
impl MeanStats {
    fn new() -> Self {
        Self {
            synth_time: 0.,
            loc: 0.,
            spec_ast: 0.,
            ast_nodes: 0.,
            ast_nodes_unsimp: 0.,
            rule_apps: 0.,
        }
    }
    pub fn calculate<'a>(many_slns: impl Iterator<Item = &'a Solved>) -> (UsedPureFns, Vec<Self>) {
        let mut count = Vec::new();
        let mut sums = Vec::new();
        let mut pure_fns = FxHashMap::default();
        for solved in many_slns {
            for (k, &v) in &solved.pure_fn_ast {
                if let Some(&pfn) = pure_fns.get(k) {
                    assert_eq!(pfn, v, "key {k}");
                } else {
                    pure_fns.insert(k.clone(), v);
                }
            }
            for (idx, sln) in solved.slns.iter().enumerate() {
                if count.len() <= idx {
                    count.push(0.)
                }
                if sums.len() <= idx {
                    sums.push(Self::new())
                }
                count[idx] += 1.;
                sums[idx].synth_time += sln.synth_time as f64;
                sums[idx].loc += sln.loc as f64;
                sums[idx].spec_ast += solved.synth_ast as f64;
                sums[idx].ast_nodes += sln.ast_nodes as f64;
                sums[idx].ast_nodes_unsimp += sln.ast_nodes_unsimp as f64;
                sums[idx].rule_apps += sln.rule_apps as f64;
            }
        }
        for (sum, count) in sums.iter_mut().zip(count) {
            sum.synth_time /= count;
            sum.loc /= count;
            sum.spec_ast /= count;
            sum.ast_nodes /= count;
            sum.ast_nodes_unsimp /= count;
            sum.rule_apps /= count;
        }
        (pure_fns, sums)
    }
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Solution {
    pub code: String,
    pub loc: usize,
    pub synth_time: u64,
    pub ast_nodes: u64,
    pub ast_nodes_unsimp: u64,
    pub rule_apps: u64,
    pub idx: usize,
}
impl Solution {
    fn new(code: &str, idx: &mut usize) -> Option<Self> {
        let loc = code.lines().count();
        if loc <= 2 {
            return None;
        };
        let loc = loc - 2;
        assert!(code.starts_with("fn "), "{code}");
        let stats_str = code.split("@|").nth(1).unwrap();
        let mut stats_str = stats_str.split('|');
        let synth_time = stats_str.next().unwrap().parse().unwrap();
        let ast_nodes = stats_str.next().unwrap().parse().unwrap();
        let ast_nodes_unsimp = stats_str.next().unwrap().parse().unwrap();
        let rule_apps = stats_str.next().unwrap().parse().unwrap();
        *idx += 1;
        Some(Self {
            code: code.to_string(),
            loc,
            synth_time,
            ast_nodes,
            ast_nodes_unsimp,
            rule_apps,
            idx: *idx - 1,
        })
    }
}

impl SuslikProgram {
    /// Timeout in ms
    pub fn solve<'tcx>(
        tcx: TyCtxt<'tcx>,
        sig: RuslikFnSig<'tcx>,
        pure_fns: &PureFnMap<'tcx>,
        extern_fns: &Vec<RuslikFnSig<'tcx>>,
        timeout: u64,
    ) -> Option<SynthesisResult> {
        let suslik_dir = Self::sbt_build_suslik();
        let params = sig.params.clone();
        let is_trivial = sig.is_trivial();
        match Self::from_fn_sig(tcx, pure_fns, extern_fns, sig) {
            Ok(sp) => sp.send_to_suslik(suslik_dir, &params, timeout),
            Err(err) => Some(SynthesisResult {
                is_trivial,
                kind: SynthesisResultKind::Unsupported(err),
            }),
        }
    }
    pub fn solve_in_thread<'tcx>(
        tx: Sender<(usize, Option<SynthesisResult>)>,
        id: usize,
        tcx: TyCtxt<'tcx>,
        sig: RuslikFnSig<'tcx>,
        pure_fns: &PureFnMap<'tcx>,
        extern_fns: &Vec<RuslikFnSig<'tcx>>,
        timeout: u64,
    ) {
        let suslik_dir = Self::sbt_build_suslik();
        let params = sig.params.clone();
        let is_trivial = sig.is_trivial();
        let sus_prog = Self::from_fn_sig(tcx, pure_fns, extern_fns, sig);
        std::thread::spawn(move || {
            let result = match sus_prog {
                Ok(sp) => sp.send_to_suslik(suslik_dir, &params, timeout),
                Err(err) => Some(SynthesisResult {
                    is_trivial,
                    kind: SynthesisResultKind::Unsupported(err),
                }),
            };
            tx.send((id, result)).unwrap();
        });
    }
    fn sbt_build_suslik() -> PathBuf {
        // Find suslik dir
        let suslik_dir = std::env::var("SUSLIK_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let mut suslik_dir = std::env::current_dir().unwrap();
                while {
                    suslik_dir.push("suslik");
                    !suslik_dir.exists()
                } {
                    suslik_dir.pop();
                    assert!(
                        suslik_dir.pop(),
                        "Failed to find suslik dir in parents of {}",
                        std::env::current_dir().unwrap().to_string_lossy()
                    );
                }
                suslik_dir
            });
        // Find suslik exe
        let mut jar_file = suslik_dir.clone();
        jar_file.extend(["target", "scala-2.12", "suslik.jar"]);
        if !jar_file.is_file() {
            // Need to assmble jar file
            println!(
                "Running `sbt assembly` since there is no executable at {}",
                jar_file.to_string_lossy()
            );
            let mut assembly = Command::new(if cfg!(windows) { "cmd" } else { "sbt" });
            if cfg!(windows) {
                assembly.arg("/c").arg("sbt");
            }
            let mut assembly = assembly
                .arg("assembly")
                .current_dir(&suslik_dir)
                .spawn()
                .expect("`sbt assembly` command failed to start");
            assembly.wait().unwrap();
            assert!(
                jar_file.is_file(),
                "Running `sbt assembly` failed to create jar file at {}",
                jar_file.to_string_lossy()
            );
        }
        suslik_dir
    }

    fn send_to_suslik(
        &self,
        suslik_dir: PathBuf,
        params: &str,
        timeout: u64,
    ) -> Option<SynthesisResult> {
        // Write program to tmp file
        let data = format!("# -c 10 -o 10 -p false\n###\n{}", self);
        let mut tmp = suslik_dir.clone();

        use rand::Rng;
        let num = rand::thread_rng().gen_range(0..10000);
        let tmpdir = std::path::PathBuf::from(format!("tmp-{}-{num}", self.synth_fn.unique_name));
        std::fs::create_dir_all(suslik_dir.join(&tmpdir)).unwrap();
        let synfile = tmpdir.join(std::path::PathBuf::from("tmp.syn"));
        tmp.push(&synfile);
        std::fs::write(tmp.clone(), data).expect("Unable to write file");
        // Run suslik on tmp file
        let mut provided_args = params
            .split(' ')
            .filter(|a| !a.is_empty())
            .map(String::from)
            .collect::<Vec<_>>();
        if provided_args.iter().all(|a| !a.contains("--solutions=")) {
            provided_args.push("--solutions=1".to_string());
        }
        let output_trace = std::env::var("RUSLIC_OUTPUT_TRACE")
            .map(|v| v.parse::<bool>().unwrap())
            .unwrap_or(false);
        if output_trace {
            let logfile = tmpdir.join(std::path::PathBuf::from("trace.json"));
            let logfile = logfile.to_str();
            provided_args.push("-j".to_string());
            provided_args.push(logfile.unwrap().to_string());
        }
        let mut child = Command::new("java")
            .arg("-Dfile.encoding=UTF-8")
            .arg("-jar")
            .arg("./target/scala-2.12/suslik.jar")
            .arg(&synfile)
            .args(provided_args)
            .current_dir(&suslik_dir)
            .stdout(Stdio::piped())
            .spawn()
            .expect("`java` command failed to start");
        let mut stdout = child.stdout.take().unwrap();
        let start = Instant::now();
        let max = Duration::from_millis(timeout);
        let (intime, exit_status, time) = match child.wait_timeout(max).expect("sbt crashed?") {
            Some(status) => (true, status, start.elapsed()),
            None => {
                // child hasn't exited yet
                child.kill().unwrap();
                println!("Failed to synthesize fn after {}ms!", timeout);
                (false, child.wait().unwrap(), max)
            }
        };
        let fail_on_unsynth = std::env::var("RUSLIC_FAIL_ON_UNSYNTH")
            .map(|v| v.parse::<bool>().unwrap())
            .unwrap_or(true);
        let unsolvable = exit_status.code().unwrap_or(0) == 2;
        let failed = !exit_status.success() && (fail_on_unsynth || !unsolvable);
        if intime && failed {
            println!(
                "suslik failed ({}) for {}",
                exit_status, self.synth_fn.fn_name
            );
            None
        } else {
            Some(if !intime {
                SynthesisResult {
                    is_trivial: self.synth_fn.is_trivial,
                    kind: SynthesisResultKind::Timeout,
                }
            } else if unsolvable {
                std::fs::remove_dir_all(suslik_dir.join(&tmpdir)).unwrap();
                SynthesisResult {
                    is_trivial: self.synth_fn.is_trivial,
                    kind: SynthesisResultKind::Unsolvable(time.as_millis() as u64),
                }
            } else {
                std::fs::remove_dir_all(suslik_dir.join(&tmpdir)).unwrap();
                let mut sln = String::new();
                use std::io::Read;
                stdout.read_to_string(&mut sln).unwrap();
                SynthesisResult {
                    is_trivial: self.synth_fn.is_trivial,
                    kind: SynthesisResultKind::Solved(Solved::new(
                        time.as_millis() as u64,
                        self.synth_ast,
                        self.pure_fn_ast.clone(),
                        sln,
                    )),
                }
            })
        }
    }

    fn from_fn_sig<'a, 'tcx>(
        tcx: TyCtxt<'tcx>,
        pure_fns: &'a PureFnMap<'tcx>,
        extern_fns: &Vec<RuslikFnSig<'tcx>>,
        sig: RuslikFnSig<'tcx>,
    ) -> Result<Self, Unsupported> {
        let def_id = sig.def_id;
        let ast_nodes = sig.ast_nodes;
        let mut map = FxHashMap::default();
        let ssig = Signature::from_fn_sig(tcx, pure_fns, sig, &mut map)?;
        let trait_fns = find_trait_fns(tcx, def_id, &ssig.tys);

        let mut efns = trait_fns
            .into_iter()
            .flat_map(|tf| {
                Signature::from_fn_sig_map(tcx, pure_fns, tf, &mut map, false)
                    .map(|sig| sig.sig)
                    .ok()
            })
            .collect::<Vec<Signature>>();
        let sgc = SGenericsCollector {
            tcx,
            synth_tys: ssig.tys,
        };
        for efn in extern_fns {
            for (gens, efn) in sgc.find_subs_for_ext_fns(efn) {
                let mut sig = Signature::from_fn_sig_map(tcx, pure_fns, efn, &mut map, false)?;
                sig.sig.unique_name = sig.sig.unique_name + "_" + &gens;
                efns.push(sig.sig);
            }
        }

        let mut res = Self {
            pred_map: map,
            extern_fns: efns,
            synth_fn: ssig.sig,
            synth_ast: ast_nodes,
            pure_fn_ast: ssig
                .used_pure_fns
                .into_iter()
                .map(|pfn| {
                    (
                        tcx.def_path_str(pfn.def_id),
                        (pfn.executable, pfn.ast_nodes),
                    )
                })
                .collect(),
        };
        res.normalize();
        Ok(res)
    }
}

pub struct SignatureSuccess<'a, 'tcx> {
    sig: Signature,
    tys: FxHashSet<rustc_middle::ty::Ty<'tcx>>,
    used_pure_fns: Vec<&'a PureFn<'tcx>>,
}

impl Signature {
    pub fn from_fn_sig<'a, 'tcx>(
        tcx: TyCtxt<'tcx>,
        pure_fns: &'a PureFnMap<'tcx>,
        sig: RuslikFnSig<'tcx>,
        map: &mut PredMap,
    ) -> Result<SignatureSuccess<'a, 'tcx>, Unsupported> {
        Self::from_fn_sig_map(tcx, pure_fns, sig, map, true)
    }
    pub fn from_fn_sig_map<'a, 'tcx>(
        tcx: TyCtxt<'tcx>,
        pure_fns: &'a PureFnMap<'tcx>,
        sig: RuslikFnSig<'tcx>,
        map: &mut PredMap,
        in_main: bool,
    ) -> Result<SignatureSuccess<'a, 'tcx>, Unsupported> {
        let use_full_names = std::env::var("RUSLIC_USE_FULL_NAMES")
            .map(|v| v.parse::<bool>().unwrap())
            .unwrap_or(false);
        let optimistically_allow_private_types =
            std::env::var("RUSLIC_OPTIMISTICALLY_ALLOW_PRIVATE_TYPES")
                .map(|v| v.parse::<bool>().unwrap())
                .unwrap_or(false);
        let mut stt = STyTranslator {
            use_full_names,
            optimistically_allow_private_types,
            tcx,
            map,
            tys: FxHashSet::default(),
            fn_id: sig.def_id,
        };
        if sig.args.iter().any(|(v, _)| v.uuid().is_empty()) {
            // functions with args (_: i32, Struct { f }: Struct) not supported
            return Err(Unsupported {
                in_main,
                reason: Reason::UnnamedArgs,
            });
        }
        let sigma = Sigma(
            sig.args
                .iter()
                .enumerate()
                .map(|(_card, (v, ty))|
            // The leading `f` is cut off in suslik:
            stt.translate_sapp(false, &v.rname(), *ty))
                .collect::<Result<_, _>>()
                .map_err(|reason| Unsupported { in_main, reason })?,
        );
        let mut pre = Assertion {
            phi: Phi::empty(),
            sigma,
        };
        let mut used_pure_fns = Vec::new();
        let mut et = ExprTranslator {
            tcx,
            pre: &mut pre,
            post: &mut Assertion::empty(),
            pure_fns,
            map: stt.map,
            call_params: None,
            is_fn_body: false,
            under_cond: Vec::new(),
            used_pure_fns: &mut used_pure_fns,
        };
        let expr = et.translate_expr(&sig.pure_pre, Vec::new(), None);
        pre.phi.0.extend(expr.flatten());

        // TODO: remove special treatment of unit
        let result = if !sig.ret.is_unit() {
            Some(
                stt.translate_sapp(false, "result", sig.ret)
                    .map_err(|reason| Unsupported { in_main, reason })?,
            )
        } else {
            None
        };
        let sigma = if let Some(result) = result {
            Sigma(vec![result])
        } else {
            Sigma::empty()
        };
        // let result = stt.translate_sapp(false, "result", sig.ret)
        //             .map_err(|reason| Unsupported { in_main, reason })?;
        // let sigma = Sigma(vec![result]);
        let mut post = Assertion {
            phi: Phi::empty(),
            sigma,
        };
        let mut et = ExprTranslator {
            tcx,
            pre: &mut pre,
            post: &mut post,
            pure_fns,
            map: stt.map,
            call_params: None,
            is_fn_body: false,
            under_cond: Vec::new(),
            used_pure_fns: &mut used_pure_fns,
        };

        // let lfts = et.translate_lfts();

        let expr = et.translate_expr(&sig.pure_post, Vec::new(), None);
        post.phi.0.extend(expr.flatten());

        // pre.phi.0.extend(lfts.flatten());
        let mut fn_name = tcx.item_name(sig.def_id).to_string();
        if !in_main
            && sig
                .args
                .first()
                .map(|arg| arg.0.uuid() != "self")
                .unwrap_or(true)
        {
            use crate::rustc_middle::ty::DefIdTree;
            if let Some(parent) = tcx.opt_parent(sig.def_id) && tcx.def_kind(parent) == DefKind::Trait {
                // let trait_name = tcx.item_name(parent);
                let trait_name = tcx.def_path_str(parent);
                let prefix = if trait_name.contains("::") && parent.is_local() { "crate::".to_string() } else { String::new() };
                fn_name = prefix + trait_name.split('<').next().unwrap() + "::" + &fn_name;
            }
        }
        let sig = Self {
            is_trivial: sig.is_trivial(),
            region_rels: outlives_relations(tcx, &sig),
            pre,
            post,
            unique_name: crate::suslik_translate::sanitize(&tcx.def_path_str(sig.def_id)),
            fn_name,
        };
        Ok(SignatureSuccess {
            sig,
            tys: stt.tys,
            used_pure_fns,
        })
    }
}

impl Expr {
    pub fn flatten(self) -> Vec<Self> {
        match self {
            Expr::BinOp(BinOp::Rust(RustBinOp::And), l, r) => {
                let mut l = l.flatten();
                l.extend(r.flatten());
                l
            }
            other => vec![other],
        }
    }
    pub fn update_vars<F: Fn(&mut String)>(&mut self, f: &F) {
        match self {
            Expr::Var(v) | Expr::Snap(_, v) | Expr::OnExpiry(_, _, v, _) => f(v),
            Expr::Tuple(_, es) => {
                for e in es {
                    e.update_vars(f)
                }
            }
            Expr::Lit(_) => (),
            Expr::BinOp(_, l, r) => {
                l.update_vars(f);
                r.update_vars(f);
            }
            Expr::UnOp(_, e) => e.update_vars(f),
            Expr::IfElse(b, t, e) => {
                b.update_vars(f);
                t.update_vars(f);
                e.update_vars(f);
            }
        }
    }
    pub fn update_result(&mut self, e_new: &Self) {
        match self {
            Expr::Var(v) | Expr::Snap(_, v) | Expr::OnExpiry(_, _, v, _) => {
                if v == "fresult" {
                    *self = e_new.clone();
                }
            }
            Expr::Tuple(_, es) => {
                for e in es {
                    e.update_result(e_new)
                }
            }
            Expr::Lit(_) => (),
            Expr::BinOp(_, l, r) => {
                l.update_result(e_new);
                r.update_result(e_new);
            }
            Expr::UnOp(_, e) => e.update_result(e_new),
            Expr::IfElse(b, t, e) => {
                b.update_result(e_new);
                t.update_result(e_new);
                e.update_result(e_new);
            }
        }
    }
    pub fn change_var<F: Fn(&str) -> Option<Expr>>(&mut self, f: &F) {
        match self {
            Expr::Var(v) => {
                if let Some(new) = f(v) {
                    *self = new;
                }
            }
            Expr::Snap(_, _) => (),
            Expr::OnExpiry(_, _, _, _) => (),
            Expr::Tuple(_, es) => {
                for e in es {
                    e.change_var(f)
                }
            }
            Expr::Lit(_) => (),
            Expr::BinOp(_, l, r) => {
                l.change_var(f);
                r.change_var(f);
            }
            Expr::UnOp(_, e) => e.change_var(f),
            Expr::IfElse(b, t, e) => {
                b.change_var(f);
                t.change_var(f);
                e.change_var(f);
            }
        }
    }
    pub fn prim_to_invs<'tcx>(value: String, kind: &'tcx TyKind<'tcx>) -> Phi {
        match *kind {
            TyKind::Bool => Phi::empty(),
            TyKind::Char => todo!(),
            TyKind::Int(i) => {
                let i = if matches!(i, rustc_middle::ty::IntTy::Isize) {
                    rustc_middle::ty::IntTy::I64
                } else {
                    i
                };
                // TODO: Scala Ints cannot parse such big values
                let (subtract, i) = match i {
                    IntTy::I8 => (0, IntTy::I8),
                    IntTy::I16 => (0, IntTy::I16),
                    IntTy::I32 => (2, IntTy::I32),
                    IntTy::I64 => (1, IntTy::I32),
                    IntTy::I128 => (0, IntTy::I32),
                    IntTy::Isize => unreachable!(),
                };
                let max_val = (1 << (i.bit_width().unwrap() - 1)) - 1 - subtract;
                Phi(vec![
                    Expr::BinOp(
                        RustBinOp::Ge.into(),
                        box Expr::Var(value.clone()),
                        box Expr::UnOp(UnOp::Neg, box max_val.into()),
                    ),
                    Expr::BinOp(
                        RustBinOp::Le.into(),
                        box Expr::Var(value),
                        box max_val.into(),
                    ),
                ])
            }
            TyKind::Uint(u) => {
                let u = if matches!(u, rustc_middle::ty::UintTy::Usize) {
                    rustc_middle::ty::UintTy::U64
                } else {
                    u
                };
                // TODO: Scala Ints cannot parse such big values
                let (subtract, u) = match u {
                    UintTy::U8 => (u128::MAX, UintTy::U8),
                    UintTy::U16 => (u128::MAX, UintTy::U16),
                    UintTy::U32 => (0, UintTy::U16),
                    UintTy::U64 => (1, UintTy::U16),
                    UintTy::U128 => (2, UintTy::U16),
                    UintTy::Usize => unreachable!(),
                };
                let max_val = (u128::checked_shl(1, u.bit_width().unwrap() as u32))
                    .unwrap_or(0)
                    .wrapping_add(subtract)
                    .into();
                Phi(vec![
                    Expr::BinOp(
                        RustBinOp::Ge.into(),
                        box Expr::Var(value.clone()),
                        box 0.into(),
                    ),
                    Expr::BinOp(RustBinOp::Le.into(), box Expr::Var(value), box max_val),
                ])
            }
            TyKind::Float(_) => todo!(),
            TyKind::Tuple(t) if t.is_empty() => Phi::empty(),
            _ => unreachable!(),
        }
    }
    pub fn _eq(self, other: Self) -> Self {
        Self::BinOp(RustBinOp::Eq.into(), box self, box other)
    }
    pub fn is_true(&self) -> bool {
        matches!(self, Expr::Lit(Lit::Bool(true)))
    }
}
impl From<u128> for Expr {
    fn from(u: u128) -> Self {
        Expr::Lit(Lit::Int(u, LitIntType::Unsuffixed))
    }
}
impl From<bool> for Expr {
    fn from(b: bool) -> Self {
        Expr::Lit(Lit::Bool(b))
    }
}
impl std::ops::BitAnd<Expr> for Expr {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        match (&self, &rhs) {
            // Optimizations:
            (Expr::Lit(Lit::Bool(true)), _) | (_, Expr::Lit(Lit::Bool(false))) => rhs,
            (_, Expr::Lit(Lit::Bool(true))) | (Expr::Lit(Lit::Bool(false)), _) => self,
            // Constructor:
            _ => Expr::BinOp(RustBinOp::And.into(), box self, box rhs),
        }
    }
}
