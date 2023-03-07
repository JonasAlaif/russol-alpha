use ruslic::suslik::{MeanStats, Reason, Solved, SynthesisResult, Unsupported};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, io::BufRead, path::PathBuf};

fn get(url: &str) -> reqwest::Result<reqwest::blocking::Response> {
    println!("Getting: {url}");
    reqwest::blocking::ClientBuilder::new()
        .user_agent("Rust Corpus - Top Crates Scrapper")
        .build()?
        .get(url)
        .send()
}

#[test]
pub fn top_crates_0() {
    top_crates_range(0..25)
}

#[test]
pub fn top_crates_1() {
    top_crates_range(25..50)
}

#[test]
pub fn top_crates_2() {
    top_crates_range(50..75)
}

#[test]
pub fn top_crates_3() {
    top_crates_range(75..100)
}

#[test]
pub fn top_crates_all() {
    top_crates_range(0..100)
}

struct KrateResults;
impl KrateResults {
    pub fn timeout_count<'a>(res: impl Iterator<Item = &'a SynthesisResult>) -> usize {
        res.filter(|res| matches!(res, SynthesisResult::Timeout))
            .count()
    }
    pub fn unsolvable_count<'a>(res: impl Iterator<Item = &'a SynthesisResult>) -> usize {
        res.filter(|res| matches!(res, SynthesisResult::Unsolvable(_)))
            .count()
    }
    pub fn unsupported<'a>(
        res: impl Iterator<Item = &'a SynthesisResult>,
    ) -> impl Iterator<Item = &'a Unsupported> {
        res.filter_map(|res| res.get_unsupported())
    }
    pub fn solved<'a>(
        res: impl Iterator<Item = &'a SynthesisResult>,
    ) -> impl Iterator<Item = &'a Solved> {
        res.filter_map(|res| res.get_solved())
    }

    pub fn reason_count<'a>(
        res: impl Iterator<Item = &'a SynthesisResult>,
    ) -> HashMap<Reason, (usize, usize)> {
        let mut counts = HashMap::new();
        for u in Self::unsupported(res) {
            let entry = counts.entry(u.reason).or_insert((0, 0));
            entry.0 += 1;
            if !u.in_main {
                entry.1 += 1;
            }
        }
        counts
    }

    pub fn summarise<'a>(res: impl Iterator<Item = &'a SynthesisResult> + Clone) {
        let timeout_count = Self::timeout_count(res.clone());
        let unsolvable_count = Self::unsolvable_count(res.clone());
        let (trivial, non_trivial): (Vec<_>, Vec<_>) =
            Self::solved(res.clone()).partition(|r| r.is_trivial);
        let (trivial_count, non_trivial_count) = (trivial.len(), non_trivial.len());
        let solved_count = trivial_count + non_trivial_count;
        let unsupported_count = Self::unsupported(res.clone()).count();
        print!("{timeout_count} & {unsolvable_count} & {unsupported_count} & {solved_count} ({trivial_count}/{non_trivial_count}) | ");

        if solved_count > 0 {
            // Trivial:
            if trivial_count > 0 {
                let mstats = MeanStats::calculate(trivial.iter().copied());
                let first = mstats.first().unwrap();
                print!(
                    "{:.1} [{:.1}/{:.1}/{:.1}/{:.1}]",
                    first.synth_time / 1000.,
                    first.loc,
                    first.ast_nodes,
                    first.ast_nodes_unsimp,
                    first.rule_apps
                );
                for mstat in mstats.into_iter().skip(1) {
                    print!(
                        "{:.1} [{:.1}/{:.1}/{:.1}/{:.1}]",
                        mstat.synth_time / 1000.,
                        mstat.loc,
                        mstat.ast_nodes,
                        mstat.ast_nodes_unsimp,
                        mstat.rule_apps
                    );
                }
            } else {
                print!("0");
            }
            print!(" / ");
            // Non-trivial:
            if non_trivial_count > 0 {
                let mstats = MeanStats::calculate(non_trivial.iter().copied());
                let first = mstats.first().unwrap();
                print!(
                    "{:.1} [{:.1}/{:.1}/{:.1}/{:.1}]",
                    first.synth_time / 1000.,
                    first.loc,
                    first.ast_nodes,
                    first.ast_nodes_unsimp,
                    first.rule_apps
                );
                for mstat in mstats.into_iter().skip(1) {
                    print!(
                        "{:.1} [{:.1}/{:.1}/{:.1}/{:.1}]",
                        mstat.synth_time / 1000.,
                        mstat.loc,
                        mstat.ast_nodes,
                        mstat.ast_nodes_unsimp,
                        mstat.rule_apps
                    );
                }
            } else {
                print!("0");
            }
            print!(" | ");
        }

        for (r, (c, _non_main)) in Self::reason_count(res) {
            print!("{r:?} {c}, ");
        }
        println!();
    }
}

pub fn top_crates_range(range: std::ops::Range<usize>) {
    let mut results = Vec::new();
    std::fs::create_dir_all("tmp").unwrap();
    let top_crates = top_crates_by_download_count(range.end);
    for krate in top_crates.into_iter().skip(range.start) {
        let version = krate.version.unwrap_or(krate.newest_version);
        let res = run_on_crate(&krate.name, &version);
        results.push((krate.name, res));
    }
    println!("\n");
    for (krate, res) in &results {
        print!("  {krate} | ");
        KrateResults::summarise(res.iter());
    }
    print!("ALL | ");
    KrateResults::summarise(results.iter().flat_map(|(_, res)| res.iter()));
    println!();

    // std::fs::remove_dir_all("tmp").unwrap();
}

fn run_on_crate(name: &str, version: &str) -> Vec<SynthesisResult> {
    let dirname = format!("./tmp/{}-{}", name, version);
    let filename = format!("{dirname}.crate");
    if !std::path::PathBuf::from(&filename).exists() {
        let dl = format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            name, version
        );
        let mut resp = get(&dl).expect("Could not fetch top crates");
        let mut file = std::fs::File::create(&filename).unwrap();
        resp.copy_to(&mut file).unwrap();
    }
    let status = std::process::Command::new("tar")
        .args(["-xf", &filename, "-C", "./tmp/"])
        .status()
        .unwrap();
    assert!(status.success());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(format!("{dirname}/Cargo.toml"))
        .unwrap();
    use std::io::Write;
    writeln!(file, "\n[workspace]").unwrap();
    let cwd = std::env::current_dir().unwrap();
    let dir = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let ruslic = cwd.join(
        ["..", "target", dir, "ruslic"]
            .iter()
            .collect::<PathBuf>(),
    );
    let suslik = cwd.join(["..", "suslik"].iter().collect::<PathBuf>());
    let timeout = std::env::var("RUSLIC_TIMEOUT").unwrap_or("300000".to_string());
    let mut child = std::process::Command::new("cargo")
        .arg("check")
        .env("RUSTC_WRAPPER", ruslic)
        .env("SUSLIK_DIR", suslik)
        .env("RUSLIC_USE_FULL_NAMES", "true")
        .env("RUSLIC_OPTIMISTICALLY_ALLOW_PRIVATE_TYPES", "true")
        .env("RUSLIC_TIMEOUT", timeout)
        .env("RUSLIC_FAIL_ON_UNSYNTH", "false")
        .env("RUSLIC_SUBST_RESULT", "true")
        .env("RUSLIC_PRINT_SLN_ABOVE", "1")
        .env("RUSLIC_SUMMARISE_JSON", "true")
        .current_dir(&dirname)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let lines = std::io::BufReader::new(stdout).lines();
    let mut results = Vec::new();
    for line in lines {
        let line = line.unwrap();
        if let Some(summary) = line.strip_prefix("###### SUMMARY @@@@@@") {
            // println!("'{summary}'");
            results.extend(serde_json::from_str::<Vec<SynthesisResult>>(summary).unwrap());
            println!();
        } else {
            println!("{line}");
        }
    }
    let status = child.wait().unwrap();
    assert!(status.success());

    let status = std::process::Command::new("cargo")
        .arg("check")
        .env("RUSTFLAGS", "--cap-lints allow")
        .current_dir(&dirname)
        .status()
        .unwrap();
    assert!(status.success());

    results
}

/// A create on crates.io.
#[derive(Debug, Deserialize, Serialize)]
struct Crate {
    #[serde(rename = "id")]
    name: String,
    #[serde(rename = "max_stable_version")]
    version: Option<String>,
    #[serde(rename = "newest_version")]
    newest_version: String,
}

/// The list of crates from crates.io
#[derive(Debug, Deserialize)]
struct CratesList {
    crates: Vec<Crate>,
}

/// Create a list of top ``count`` crates.
fn top_crates_by_download_count(mut count: usize) -> Vec<Crate> {
    const PAGE_SIZE: usize = 100;
    let page_count = count / PAGE_SIZE + 2;
    let mut sources = Vec::new();
    for page in 1..page_count {
        let url = format!(
            "https://crates.io/api/v1/crates?page={}&per_page={}&sort=downloads",
            page, PAGE_SIZE
        );
        let resp = get(&url).expect("Could not fetch top crates");
        assert!(
            resp.status().is_success(),
            "Response status: {}",
            resp.status()
        );
        let page_crates: CratesList = match serde_json::from_reader(resp) {
            Ok(page_crates) => page_crates,
            Err(e) => panic!("Invalid JSON {e}"),
        };
        sources.extend(page_crates.crates.into_iter().take(count));
        count -= std::cmp::min(PAGE_SIZE, count);
    }
    sources
}
