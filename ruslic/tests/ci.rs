use std::{fmt::Display, path::PathBuf};

use ruslic::suslik::{MeanStats, Solved, SynthesisResult};

struct Category {
    dir: String,
    results: Vec<(String, SynthesisResult)>,
    children: Vec<Category>,
    // solutions: Vec<Solved>,
}
impl Category {
    fn run_tests_in_dir(dir: PathBuf, timeout: u64) -> Self {
        let mut cat = Self {
            dir: dir.file_name().unwrap().to_string_lossy().to_string(),
            results: Vec::new(),
            children: Vec::new(),
            // solutions: Vec::new(),
        };
        for path in std::fs::read_dir(dir).unwrap() {
            let path = path.unwrap();
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
                let results = Self::run_tests_in_dir(path.path(), timeout);
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
        let solved = self.solved().count();
        write!(f, "# {0} ({0} & {solved}", self.dir)?;
        if solved > 0 {
            let mean_stats = &MeanStats::calculate(self.solved())[0];
            write!(
                f,
                " & LOC {:.1} & AN {:.1} & SN {:.1} & USN {:.1} & RA {:.1} & T {:.1}",
                mean_stats.loc,
                mean_stats.spec_ast,
                mean_stats.ast_nodes,
                mean_stats.ast_nodes_unsimp,
                mean_stats.rule_apps,
                mean_stats.synth_time / 1000.
            )?;
        }
        write!(f, ")")?;
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
        for child in &self.children {
            let child = child.to_string();
            write!(f, "\n  ")?;
            write!(f, "{}", child.replace('\n', "\n  "))?;
        }
        Ok(())
    }
}

#[test]
fn all_tests() {
    let timeout = std::env::var("RUSLIC_TIMEOUT").ok().and_then(
        |t| t.parse().ok()
    ).unwrap_or(300_000);
    // std::env::set_var("RUSLIC_FAIL_ON_UNSYNTH", "false");
    std::env::set_var("RUSLIC_OUTPUT_TRACE", "false");
    let results = Category::run_tests_in_dir(PathBuf::from("./tests/synth/"), timeout);
    let max_ms = format_ms(results.max_ms());
    println!("### Measured timings (max {max_ms}) ###");
    println!("{results}");
    println!("#######################################");
    // Make sure this gets printed in the correct order in GitHub:
    std::thread::sleep(std::time::Duration::from_millis(1000));
    if results.errors().count() > 0 {
        let err: Vec<_> = results.errors().map(|err| &err.0).collect();
        panic!("Tests {:?} errored or exceeded timeout of {timeout}!", err);
    }
}

fn format_ms(ms: u64) -> String {
    format!("{}_{:03}ms", ms / 1000, ms % 1000)
}
