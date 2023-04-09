use ruslic::suslik::{MeanStats, Reason, Solved, SynthesisResult, Unsupported};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result},
    io::BufRead,
    path::PathBuf,
};

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

fn get_cached_crates() -> Vec<String> {
    let mut paths: Vec<_> = std::fs::read_dir("./tests/top_100_crates")
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    paths.sort_by_key(|dir| dir.path());

    let mut cached_crates = Vec::new();
    for path in paths {
        if path.file_type().unwrap().is_dir() {
            continue;
        }
        let file_name = path.file_name();
        if let Some(file_name) = file_name.to_str().unwrap().strip_suffix(".crate") {
            cached_crates.push(file_name.to_string());
        }
    }
    cached_crates
}

#[test]
pub fn top_crates_cached() {
    let mut results = Vec::new();
    for krate in get_cached_crates() {
        let dirname = format!("./tests/top_100_crates/{krate}");
        let res = run_on_crate(&dirname, true);
        results.push((krate, res));
    }
    let results = AllResults::new(results);
    let results_str = results.to_string();
    println!("\n\n{results_str}");
    std::fs::write("./tests/crates-results.txt", results_str).expect("Unable to results to file!");
}

#[test]
pub fn prefetch_cached() {
    for krate in get_cached_crates() {
        let dirname = format!("./tests/top_100_crates/{krate}");
        extract_crate(&dirname);
        let status = std::process::Command::new("cargo")
            .arg("check")
            .current_dir(&dirname)
            .status()
            .unwrap();
        assert!(status.success());
        std::fs::remove_dir_all(&dirname).unwrap();
    }
}

struct KrateResults<'a, T: Iterator<Item = &'a SynthesisResult> + Clone> {
    res: T,
    is_eval: bool,
}

impl<'a, T: Iterator<Item = &'a SynthesisResult> + Clone> KrateResults<'a, T> {
    fn count(i: impl Iterator<Item = &'a SynthesisResult>) -> (usize, usize) {
        let (mut trivial, mut non) = (0, 0);
        for res in i {
            if res.is_trivial {
                trivial += 1;
            } else {
                non += 1;
            }
        }
        (trivial, non)
    }
    pub fn timeout_count(&self) -> (usize, usize) {
        let res = self.res.clone();
        Self::count(res.filter(|res| res.is_timeout()))
    }
    pub fn unsolvable_count(&self) -> (usize, usize) {
        let res = self.res.clone();
        Self::count(res.filter(|res| res.get_unsolvable().is_some()))
    }
    pub fn unsupported_count(&self) -> (usize, usize) {
        let res = self.res.clone();
        Self::count(res.filter(|res| res.get_unsupported().is_some()))
    }

    pub fn unsupported(&self) -> impl Iterator<Item = (bool, &'a Unsupported)> {
        let res = self.res.clone();
        res.filter_map(|res| res.get_unsupported().map(|u| (res.is_trivial, u)))
    }
    pub fn solved(&self) -> impl Iterator<Item = (bool, &'a Solved)> {
        let res = self.res.clone();
        res.filter_map(|res| res.get_solved().map(|s| (res.is_trivial, s)))
    }

    pub fn reason_count(&self) -> HashMap<Reason, (usize, usize)> {
        let mut counts = HashMap::new();
        for (_, u) in self.unsupported() {
            let entry = counts.entry(u.reason).or_insert((0, 0));
            entry.0 += 1;
            if !u.in_main {
                entry.1 += 1;
            }
        }
        counts
    }
}

impl<'a, T: Iterator<Item = &'a SynthesisResult> + Clone> Display for KrateResults<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let (to_t, to_n) = self.timeout_count();
        let to_sum = to_t + to_n;
        let (unsv_t, unsv_n) = self.unsolvable_count();
        let unsv_sum = unsv_t + unsv_n;
        let (failed_t, failed_n) = (to_t + unsv_t, to_n + unsv_n);
        let failed_sum = failed_t + failed_n;

        let (slns_t, slns_n): (Vec<_>, Vec<_>) = self.solved().partition(|r| r.0);
        let (slvd_t, slvd_n) = (slns_t.len(), slns_n.len());
        let slvd_sum = slvd_t + slvd_n;

        let (unsup_t, unsup_n) = self.unsupported_count();
        let unsup_sum = unsup_t + unsup_n;

        write!(f, "Unsupported {unsup_sum} (triv {unsup_t}, non-triv {unsup_n}) Failed {failed_sum} (triv {failed_t}, non-triv {failed_n}) Solved {slvd_sum} (triv {slvd_t}, non-triv {slvd_n})")?;

        if slvd_sum > 0 {
            fn print_mstats(
                f: &mut Formatter<'_>,
                mstats: Vec<MeanStats>,
                is_eval: bool,
            ) -> Result {
                let mut print_stat = move |mstat: &MeanStats, is_first: bool| -> Result {
                    let sep = if is_first { "" } else { ", " };
                    write!(
                        f,
                        "{sep}Time {:.2}s LOC {:.2} Rules {:.2}",
                        mstat.synth_time / 1000.,
                        mstat.loc,
                        mstat.rule_apps,
                    )?;
                    if !is_eval {
                        write!(
                            f,
                            " sln nodes {:.2} sln unsimp nodes {:.2}",
                            mstat.ast_nodes, mstat.ast_nodes_unsimp
                        )?;
                    }
                    Ok(())
                };
                let first = mstats.first().unwrap();
                print_stat(first, true)?;
                for mstat in mstats.into_iter().skip(1) {
                    print_stat(&mstat, false)?;
                }
                Ok(())
            }

            // Trivial:
            write!(f, " | Trivial: ")?;
            if slvd_t > 0 {
                let (_, mstats) = MeanStats::calculate(slns_t.iter().map(|(_, s)| s).copied());
                print_mstats(f, mstats, self.is_eval)?;
            } else {
                write!(f, "0")?;
            }
            write!(f, " / Non-trivial: ")?;
            // Non-trivial:
            if slvd_n > 0 {
                let (_, mstats) = MeanStats::calculate(slns_n.iter().map(|(_, s)| s).copied());
                print_mstats(f, mstats, self.is_eval)?;
            } else {
                write!(f, "0")?;
            }
        }
        write!(f, " | Falied due to: Timeout {to_sum} (triv {to_t}, non-triv {to_n}), Unsolvable {unsv_sum} (triv {unsv_t}, non-triv {unsv_n})")?;
        write!(f, " | Unsupported due to: ")?;
        for (r, (c, _non_main)) in self.reason_count() {
            write!(f, "{r:?} {c}, ")?;
        }
        writeln!(f)
    }
}

pub fn top_crates_range(range: std::ops::Range<usize>) {
    let mut results = Vec::new();
    std::fs::create_dir_all("./tests/top_100_crates").unwrap();
    let top_crates = top_crates_by_download_count(range.end);
    for krate in top_crates.into_iter().skip(range.start) {
        let version = krate.version.unwrap_or(krate.newest_version);
        let dirname = download_crate(&krate.name, &version);
        let res = run_on_crate(&dirname, false);
        results.push((krate.name, res));
    }
    let results = AllResults::new(results);
    let results_str = results.to_string();
    println!("\n\n{results_str}");
    std::fs::write("./tests/crates-results.txt", results_str).expect("Unable to results to file!");
    // std::fs::remove_dir_all("./tests/top_100_crates").unwrap();
}

struct AllResults(Vec<(String, Vec<SynthesisResult>)>, bool);
impl AllResults {
    pub fn new(results: Vec<(String, Vec<SynthesisResult>)>) -> Self {
        let is_eval = std::env::var("RUSLIC_EVAL")
            .ok()
            .map(|s| s.parse::<bool>().unwrap())
            .unwrap_or(false);
        Self(results, is_eval)
    }
}
impl Display for AllResults {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if !self.1 {
            for (krate, res) in &self.0 {
                let res = KrateResults {
                    res: res.iter(),
                    is_eval: self.1,
                };
                write!(f, "  {krate} | {res}")?;
            }
        }
        let all = KrateResults {
            res: self.0.iter().flat_map(|(_, res)| res.iter()),
            is_eval: self.1,
        };
        write!(f, "ALL {} | {all}", self.0.len())
    }
}

fn download_crate(name: &str, version: &str) -> String {
    let dirname = format!("./tests/top_100_crates/{name}-{version}");
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
    dirname
}

fn extract_crate(dirname: &str) {
    let status = std::process::Command::new("tar")
        .args([
            "-xf",
            &format!("{dirname}.crate"),
            "-C",
            "./tests/top_100_crates/",
        ])
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
}
fn cargo_cmd(offline: bool) -> std::process::Command {
    let mut cc = std::process::Command::new("cargo");
    cc.arg("check");
    if offline {
        cc.arg("--offline");
    }
    cc
}
fn run_on_crate(dirname: &str, offline: bool) -> Vec<SynthesisResult> {
    extract_crate(dirname);
    let cwd = std::env::current_dir().unwrap();
    let dir = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let ruslic = cwd.join(["..", "target", dir, "ruslic"].iter().collect::<PathBuf>());
    let suslik = cwd.join(["..", "suslik"].iter().collect::<PathBuf>());
    let timeout = std::env::var("RUSLIC_TIMEOUT").unwrap_or("300000".to_string());
    // let mt = std::env::var("RUSLIC_THREAD_COUNT").unwrap_or("12".to_string());
    let mut child = cargo_cmd(offline)
        .env("RUSTC_WRAPPER", ruslic)
        .env("SUSLIK_DIR", suslik)
        .env("RUSLIC_USE_FULL_NAMES", "true")
        .env("RUSLIC_OPTIMISTICALLY_ALLOW_PRIVATE_TYPES", "true")
        .env("RUSLIC_TIMEOUT", timeout)
        .env("RUSLIC_FAIL_ON_UNSYNTH", "false")
        .env("RUSLIC_SUBST_RESULT", "true")
        .env("RUSLIC_PRINT_SLN_ABOVE", "1")
        .env("RUSLIC_SUMMARISE_JSON", "true")
        .env("RUSLIC_OUTPUT_TRACE", "true")
        .env("RUSLIC_SUMMARISE", "true")
        // .env("RUSLIC_THREAD_COUNT", mt)
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

    // Check that everything compiles
    let status = cargo_cmd(offline)
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
