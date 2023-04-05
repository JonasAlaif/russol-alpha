use std::{fmt::Display, path::PathBuf};

use ruslic::suslik::{MeanStats, Solved, SynthesisResult};

struct Category {
    dir: String,
    results: Vec<(String, SynthesisResult)>,
    children: Vec<Category>,
    depth: u32,
}
impl Category {
    fn run_tests_in_dir(dir: PathBuf, timeout: u64, depth: u32) -> Self {
        let mut cat = Self {
            dir: dir.file_name().unwrap().to_string_lossy().to_string(),
            results: Vec::new(),
            children: Vec::new(),
            depth,
        };
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        paths.sort_by_key(|dir| dir.path());

        for path in paths {
            if path.file_type().unwrap().is_file() {
                let filename = path.file_name();
                let filename = filename.to_string_lossy();
                if filename.ends_with(".rs") {
                    println!(
                        "Attempting synthesis for: {}",
                        path.path().to_string_lossy()
                    );
                    if let Ok(res) = ruslic::run_on_file(
                        vec![
                            "/name/of/binary".to_string(),
                            path.path().to_string_lossy().to_string(),
                        ],
                        timeout,
                        false,
                    ) {
                        cat.results.extend(
                            res.into_iter()
                                .map(|(k, v)| (filename.to_string() + "::" + &k, v)),
                        );
                        println!();
                    } else {
                        panic!(
                            "### Error when executing: {} ###",
                            path.path().to_string_lossy()
                        );
                    }
                }
            } else {
                let results = Self::run_tests_in_dir(path.path(), timeout, depth + 1);
                cat.children.push(results)
            }
        }
        // cat.solutions.extend(cat.results.iter().filter_map(|(_, res)| res.get_solved()).cloned());
        // cat.solutions.extend(cat.children.iter().flat_map(|(_, res)| &res.solutions).cloned());
        cat
    }
    fn solutions(&self) -> Box<dyn Iterator<Item = &(String, SynthesisResult)> + '_> {
        Box::new(
            self.results
                .iter()
                .chain(self.children.iter().flat_map(|res| res.solutions())),
        )
    }
    fn solved(&self) -> impl Iterator<Item = &Solved> + '_ {
        self.solutions().flat_map(|sln| sln.1.get_solved())
    }
    fn errors(&self) -> impl Iterator<Item = &(String, SynthesisResult)> + '_ {
        self.solutions().filter(|sln| sln.1.get_solved().is_none())
    }
    fn max_ms(&self) -> u64 {
        self.solved()
            .map(|sln| sln.slns.first().unwrap().synth_time)
            .max()
            .unwrap_or(0)
    }
}

impl Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let is_eval = std::env::var("RUSLIC_EVAL")
            .ok()
            .map(|s| s.parse::<bool>().unwrap())
            .unwrap_or(false);
        if is_eval && self.depth > 2 {
            return Ok(());
        }
        let solved = self.solved().count();
        write!(f, "\n{: <22} Solved {solved}", self.dir)?;
        if solved > 0 {
            let (pure_fns, mean_stats) = MeanStats::calculate(self.solved());
            let mean_stats = &mean_stats[0];
            let pure_fn_nodes = pure_fns
                .values()
                .filter(|(exec, _)| !exec)
                .fold(0, |acc, (_, n)| acc + n);
            let pure_fn_nodes = pure_fn_nodes as f64 / solved as f64;
            let ann_overhead = mean_stats.ast_nodes / (mean_stats.spec_ast + pure_fn_nodes);
            write!(
                f,
                " \t Time {:.1}s \t SOL rules {:.2} \t Rust LOC {:.2} \t Code/Spec {:.2} {} Sln nodes {:.2} \t Ann nodes {:.2} \t Non-exec pure fn nodes {:.2}",
                mean_stats.synth_time / 1000.,
                mean_stats.rule_apps,
                mean_stats.loc,
                ann_overhead,
                if is_eval { "| Overhead data:" } else { "\t" },
                mean_stats.ast_nodes,
                mean_stats.spec_ast,
                pure_fn_nodes,
                // mean_stats.ast_nodes_unsimp,
            )?;
            if !is_eval {
                // let pure_fns: std::collections::HashMap<_, _> = pure_fns.iter().filter(|(_, (exec, _))| !exec).collect();
                if !pure_fns.is_empty() {
                    write!(
                        f,
                        " \t | \t Pure functions (\"name\": (executable, ast_nodes)): {pure_fns:?}"
                    )?;
                }
            }
        }
        if !is_eval {
            for (r, v) in &self.results {
                write!(f, "\n  {} - ", r)?;
                if let Some(sln) = v.get_solved() {
                    let first = sln.slns.first().unwrap();
                    write!(
                        f,
                        "{} [{}/{}/{}/{}]",
                        format_ms(first.synth_time),
                        first.loc,
                        first.ast_nodes,
                        first.ast_nodes_unsimp,
                        first.rule_apps
                    )?;
                    for sln in sln.slns.iter().skip(1) {
                        write!(
                            f,
                            ",  {} [{}/{}/{}/{}]",
                            format_ms(sln.synth_time),
                            sln.loc,
                            sln.ast_nodes,
                            sln.ast_nodes_unsimp,
                            sln.rule_apps
                        )?;
                    }
                    write!(
                        f,
                        " | spec_ast: {}, pfn_ast: {:?}",
                        sln.synth_ast, sln.pure_fn_ast
                    )?;
                } else {
                    write!(f, "{:?}", v)?;
                }
            }
        }
        for child in &self.children {
            let child = child.to_string();
            write!(f, "{}", child.replace('\n', "\n  "))?;
        }
        Ok(())
    }
}

#[test]
fn all_tests() {
    let timeout = std::env::var("RUSLIC_TIMEOUT")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(300_000);
    let is_eval = std::env::var("RUSLIC_EVAL")
        .ok()
        .map(|s| s.parse::<bool>().unwrap())
        .unwrap_or(false);

    let results = if is_eval {
        all_tests_eval(timeout)
    } else {
        Category::run_tests_in_dir(PathBuf::from("./tests/synth/"), timeout, 0)
    };
    let max_ms = format_ms(results.max_ms());
    let results_str = format!("### Measured timings (max {max_ms}) ###{results}\n#######################################\n");
    print!("{results_str}");
    std::fs::write("./tests/ci-results.txt", results_str).expect("Unable to results to file!");
    // Make sure this gets printed in the correct order in GitHub:
    std::thread::sleep(std::time::Duration::from_millis(1000));
    if results.errors().count() > 0 {
        let err: Vec<_> = results.errors().map(|err| &err.0).collect();
        panic!("Tests {:?} errored or exceeded timeout of {timeout}!", err);
    }
}

fn all_tests_eval(timeout: u64) -> Category {
    Category::run_tests_in_dir(PathBuf::from("./tests/synth/paper"), timeout, 0)
}

fn format_ms(ms: u64) -> String {
    format!("{}_{:03}ms", ms / 1000, ms % 1000)
}
