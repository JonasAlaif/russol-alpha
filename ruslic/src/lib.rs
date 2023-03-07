#![feature(rustc_private)]
#![feature(iter_intersperse)]
#![feature(box_patterns)]
#![feature(if_let_guard)]
#![feature(let_chains)]
#![feature(box_syntax)]
#![feature(assert_matches)]
#![allow(clippy::needless_lifetimes)]

extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_attr;
extern crate rustc_const_eval;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_pretty;
extern crate rustc_infer;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_type_ir;
extern crate rustc_typeck;

mod constant;
mod contract_translator;
mod hir_translator;
mod interner;
mod ruslik_pure;
mod ruslik_pure_helpers;
mod ruslik_ssl;
mod ruslik_types;
mod src_replace;
mod subst_generics;
pub mod suslik;
mod suslik_normalize;
mod suslik_translate;
mod trait_bounds;
// mod fnsig_regions;

use rustc_data_structures::fx::FxHashMap;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::{interface::Compiler, Queries};
use suslik::SynthesisResult;

struct CompilerCallbacks {
    is_cargo: bool,
    timeout: u64,
    timings: FxHashMap<String, SynthesisResult>,
}
impl Callbacks for CompilerCallbacks {
    fn after_expansion<'tcx>(
        &mut self,
        compiler: &Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        compiler.session().abort_if_errors();

        unsafe {
            crate::ruslik_ssl::VARS = Some(rustc_data_structures::fx::FxHashSet::default());
            crate::ruslik_ssl::UNIFS = Some(rustc_data_structures::fx::FxHashSet::default());
        }
        if false {
            let (krate, _resolver, _lint_store) = &mut *queries.expansion().unwrap().peek_mut();
            rustc_driver::pretty::print_after_parsing(
                compiler.session(),
                compiler.input(),
                krate,
                rustc_session::config::PpMode::Source(rustc_session::config::PpSourceMode::Normal),
                None,
            );
        }

        queries.prepare_outputs().unwrap();

        queries.global_ctxt().unwrap().peek_mut().enter(|tcx| {
            match crate::interner::intern(tcx, self.timeout) {
                Some(times) => self.timings = times,
                None => {
                    panic!("error");
                    // tcx.sess.err(e.0);
                }
            }
        });
        if self.is_cargo {
            Compilation::Continue
        } else {
            Compilation::Stop
        }
    }
}

/// Adds the correct --sysroot option.
fn sys_root() -> Vec<String> {
    let home = option_env!("RUSTUP_HOME");
    let toolchain = option_env!("RUSTUP_TOOLCHAIN");
    let sysroot = format!("{}/toolchains/{}", home.unwrap(), toolchain.unwrap());
    vec!["--sysroot".into(), sysroot]
}

/// Pass rustc arguments in args (namely, path to rust file). Timeout is per function to be synthesized.
pub fn run_on_file(
    mut args: Vec<String>,
    timeout: u64,
    is_cargo: bool,
) -> Result<FxHashMap<String, SynthesisResult>, rustc_errors::ErrorGuaranteed> {
    let current_dir = std::env::current_exe().unwrap();
    let current_dir = current_dir.parent().unwrap();
    // Back out once more if running CI
    let current_dir = if current_dir.ends_with("deps") {
        current_dir.parent().unwrap()
    } else {
        current_dir
    };

    if !is_cargo {
        args.push("--crate-type=lib".into());
    }
    args.extend(["-A".into(), "dead_code".into()]);
    args.extend(["-A".into(), "unused_variables".into()]);
    args.extend(sys_root());
    if !is_cargo {
        args.push("--edition=2021".into());
    }
    std::env::set_var(
        "LD_LIBRARY_PATH",
        std::env::current_exe().unwrap().as_os_str(),
    );
    std::env::set_var(
        "DYLD_LIBRARY_PATH",
        std::env::current_exe().unwrap().as_os_str(),
    );
    let russol_contracts = current_dir.join("librussol_contracts.rlib");
    args.extend([
        "--extern".into(),
        format!(
            "russol_contracts={}",
            russol_contracts.as_os_str().to_str().unwrap()
        ),
    ]);

    args.push("-L".into());
    args.push(format!(
        "dependency={}",
        current_dir.join("deps").as_os_str().to_str().unwrap()
    ));

    // println!("Running with args: {:?}", args);
    let mut cc = CompilerCallbacks {
        is_cargo,
        timeout,
        timings: FxHashMap::default(),
    };
    RunCompiler::new(&args, &mut cc).run()?;
    Ok(cc.timings)
}
