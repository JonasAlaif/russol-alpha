use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::DefId;
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

    let multithreaded = std::env::var("RUSLIC_THREAD_COUNT")
        .map(|v| v.parse::<usize>().unwrap())
        .unwrap_or(8);
    if multithreaded > 1 {
        solve_multithreaded(tcx, timeout, translator, multithreaded)
    } else {
        solve(tcx, timeout, translator)
    }
}

pub fn solve<'tcx>(
    tcx: TyCtxt<'tcx>,
    timeout: u64,
    translator: HirTranslator<'tcx>,
) -> Option<FxHashMap<String, SynthesisResult>> {
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
        handle_result(result, &mut times, tcx, def_id, name, multifn);
    }

    Some(times)
}

pub fn solve_multithreaded<'tcx>(
    tcx: TyCtxt<'tcx>,
    timeout: u64,
    translator: HirTranslator<'tcx>,
    thread_count: usize,
) -> Option<FxHashMap<String, SynthesisResult>> {
    let multifn = translator.impure_fns.len() > 1;
    let only_synth = translator.impure_fns.iter().any(|(s, _)| *s);
    let impure_fns = if only_synth {
        translator
            .impure_fns
            .into_iter()
            .filter(|(s, _)| *s)
            .collect()
    } else {
        translator.impure_fns
    };
    let mut results: Vec<(DefId, Option<SynthesisResult>)> = Vec::new();
    let (tx, rx) = std::sync::mpsc::channel();
    for (_, sig) in impure_fns.into_iter() {
        results.push((sig.def_id, None));
        if results.len() > thread_count {
            let (idx, result) = rx.recv().unwrap();
            let results: &mut (_, _) = &mut results[idx];
            results.1 = result;
        }
        SuslikProgram::solve_in_thread(
            tx.clone(),
            results.len() - 1,
            tcx,
            sig,
            &translator.pure_fns,
            &translator
                .extern_fns
                .iter()
                .map(|ef| (*ef).clone())
                .collect(),
            timeout,
        );
    }
    for _ in 0..std::cmp::min(results.len(), thread_count) {
        let (idx, result) = rx.recv().unwrap();
        results[idx].1 = result;
    }

    let mut times = FxHashMap::default();
    for (def_id, result) in results.into_iter() {
        let name = tcx.def_path_str(def_id);
        let result = result.unwrap();
        handle_result(result, &mut times, tcx, def_id, name, multifn);
    }

    Some(times)
}

pub fn handle_result(
    result: SynthesisResult,
    times: &mut FxHashMap<String, SynthesisResult>,
    tcx: TyCtxt,
    def_id: DefId,
    name: String,
    multifn: bool,
) {
    if let Some(sln) = result.get_solved() {
        sln.print();
    }

    // eprintln!("Synth for {:?} result: {:?}", def_id, result);
    let subst_result = std::env::var("RUSLIC_SUBST_RESULT")
        .map(|v| v.parse::<bool>().unwrap())
        .unwrap_or(false);
    if let Some(sln) = result.get_solved() && subst_result {
        let sln_lines = sln.slns[0].loc;
        let sln = sln.slns[0].code.lines().skip(1).take(sln_lines).fold("\n".to_string(), |acc, line| acc + line + "\n");
        replace_with_sln(tcx, def_id, sln, multifn);
    }
    times.insert(name, result);
}
