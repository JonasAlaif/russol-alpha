#![feature(rustc_private)]
extern crate rustc_driver;
extern crate rustc_errors;

use ruslic::suslik::{SynthesisResult, SynthesisResultKind};
use rustc_errors::ErrorGuaranteed;

fn main() -> Result<(), ErrorGuaranteed> {
    rustc_driver::catch_fatal_errors(|| {
        match filter_args() {
            // Skip synth
            (args, _, true) => {
                let status = std::process::Command::new("rustc")
                    .args(args.into_iter().skip(1))
                    .status()
                    .unwrap();
                assert!(status.success());
            }
            // Do synth
            (args, is_cargo, false) => {
                let timeout = std::env::var("RUSLIC_TIMEOUT")
                    .map(|v| v.parse::<u64>().unwrap())
                    .unwrap_or(1_000_000);
                if let Ok(res) = ruslic::run_on_file(args, timeout, is_cargo) {
                    summarise(res.values().collect());
                }
            }
        }
    })
}

fn filter_args() -> (Vec<String>, bool, bool) {
    let mut is_cargo = false;
    let mut crate_name = false;
    let mut is_build_script = false;
    let args = std::env::args()
        .filter(|arg| {
            if crate_name {
                assert!(!is_build_script);
                is_build_script = arg == "build_script_main" || arg == "build_script_build";
            }
            crate_name = arg == "--crate-name";

            is_cargo = is_cargo || arg == "rustc";
            arg != "rustc"
        })
        .collect();
    let skip_synth =
        (std::env::var("CARGO_PRIMARY_PACKAGE").is_err() && is_cargo) || is_build_script;
    (args, is_cargo, skip_synth)
}

fn summarise(res: Vec<&SynthesisResult>) {
    if !std::env::var("RUSLIC_SUMMARISE")
        .map(|v| v.parse().unwrap())
        .unwrap_or(false)
    {
        return;
    }
    let (mut unsupported, mut unsolvable, mut timeout, mut solved) = (0, 0, 0, Vec::new());
    for res in res.iter() {
        match &res.kind {
            SynthesisResultKind::Unsupported(_) => unsupported += 1,
            SynthesisResultKind::Unsolvable(_) => unsolvable += 1,
            SynthesisResultKind::Timeout => timeout += 1,
            SynthesisResultKind::Solved(s) => {
                for (idx, sln) in s.slns.iter().enumerate() {
                    if solved.len() <= idx {
                        solved.push((0, 0, 0));
                    }
                    solved[idx].0 += 1;
                    solved[idx].1 += sln.loc;
                    solved[idx].2 += sln.synth_time;
                }
            }
        }
    }
    println!("Unsupported: {unsupported}\nUnsolvable: {unsolvable}\nTimeout: {timeout}");
    print!("Solved: ");
    for (solved, lines, time) in solved.iter() {
        print!(" {solved} (loc {lines}, time {time}),");
    }
    if solved.is_empty() {
        println!("0");
    } else {
        println!();
    }
    let json = std::env::var("RUSLIC_SUMMARISE_JSON")
        .map(|v| v.parse::<bool>().unwrap())
        .unwrap_or(false);
    if json {
        let serialized = serde_json::to_string(&res).unwrap();
        assert!(!serialized.contains('\n'));
        println!("###### SUMMARY @@@@@@{serialized}");
    }
}
