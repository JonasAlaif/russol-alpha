use rustc_data_structures::fx::FxHashMap;
use rustc_middle::ty::TyCtxt;

use crate::{
    hir_translator::HirTranslator,
    src_replace::replace_with_sln,
    suslik::{SuslikProgram, SynthesisResult},
};

pub fn intern(tcx: TyCtxt, timeout: u64) -> Option<FxHashMap<String, SynthesisResult>> {
    rustc_typeck::check_crate(tcx).ok()?;
    tcx.hir()
        .par_body_owners(|def_id| tcx.ensure().check_match(def_id.to_def_id()));
    tcx.hir().par_for_each_module(|module| {
        tcx.ensure().check_mod_privacy(module);
    });
    if tcx.sess.has_errors().is_some() {
        return None;
    }

    let mut translator = HirTranslator::new(tcx);
    for def_id in tcx.hir().body_owners() {
        let def_id = def_id.to_def_id();
        tcx.ensure().check_match(def_id);
        if tcx.sess.has_errors().is_some() {
            return None;
        }
        // println!("Translating {:?}", def_id);
        translator.translate(def_id);
    }

    let multifn = translator.impure_fns.len() > 1;
    let mut times = FxHashMap::default();
    let only_synth = translator.impure_fns.iter().any(|(s, _)| *s);
    for (synth, sig) in translator.impure_fns.into_iter() {
        if only_synth && !synth {
            continue;
        }
        let def_id = sig.def_id;
        let name = tcx.def_path_str(def_id);
        let result = SuslikProgram::solve(
            tcx,
            sig,
            &translator.pure_fns,
            &translator
                .extern_fns
                .iter()
                .map(|ef| (*ef).clone())
                .collect(),
            timeout,
        )?;
        // eprintln!("Synth for {:?} result: {:?}", def_id, result);
        let subst_result = std::env::var("RUSLIC_SUBST_RESULT")
            .map(|v| v.parse::<bool>().unwrap())
            .unwrap_or(false);
        if let SynthesisResult::Solved(sln) = &result && subst_result {
            let sln_lines = sln.slns[0].loc;
            let sln = sln.slns[0].code.lines().skip(1).take(sln_lines).fold("\n".to_string(), |acc, line| acc + line + "\n");
            replace_with_sln(tcx, def_id, sln, multifn);
        }
        times.insert(name, result);
    }

    Some(times)
}
