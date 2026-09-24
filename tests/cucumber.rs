//! thimbl verification harness — cucumber-rs entry point.
//!
//! Composition (all verified against cucumber 0.23.0 APIs):
//! * stdout human output with summary + JSON evidence tee'd to
//!   `target/cucumber.json` (one JSON file per run — the focused runner
//!   parses it to print per-target failure evidence; `THIMBL_CUKE_JSON`
//!   overrides the path);
//! * `repeat_skipped()` so retried scenarios survive the filter;
//! * `fail_on_skipped()` so an undefined step fails the run (exit != 0);
//! * `with_default_cli()` so cucumber-rs does NOT parse process args/env
//!   itself. This is load-bearing twice over:
//!   1. cucumber-rs' clap derive marks `--tags` as conflicting with
//!      `--name`, and an env-provided `CUCUMBER_FILTER_TAGS` value
//!      triggers the same conflict — the RIGGING.md focused command
//!      combines both, so CLI parsing must be skipped;
//!   2. `--name` is applied as a strict regex against scenario names,
//!      and scenario "Long format mirrors finger(1) fields" contains
//!      regex metacharacters — the raw expansion can never match. The
//!      harness therefore owns `--name` parsing (regex first, exact
//!      literal fallback) and the zero-scenario guard: a focused run
//!      that selects 0 scenarios fails loudly with exit 2.
//!
//! Scenario selection applied by `filter_run_and_exit`'s predicate:
//! `CUCUMBER_FILTER_TAGS` tag expression AND (when given) the `--name`
//! scenario filter.

use std::sync::atomic::{AtomicUsize, Ordering};

use cucumber::tag::Ext as _;
use cucumber::{World, WriterExt as _};

#[path = "cucumber/mod.rs"]
mod steps;
mod support;

use support::FingerWorld;
use support::evidence::JsonEvidencePath;

/// Scenarios accepted by the combined filter during this run.
static MATCHED_SCENARIOS: AtomicUsize = AtomicUsize::new(0);

#[tokio::main]
async fn main() {
    // Own scenario selection: tag expression from the env var, name
    // filter from the `--name` argv (cucumber's own CLI is disabled).
    let tag_expr = tag_expression_from_env();
    let name_filter = name_filter_from_args();
    let name_in_closure = name_filter.clone();

    let json_path = JsonEvidencePath::from_env();
    let json_file = std::fs::File::create(&json_path.0).expect("create JSON evidence file");
    let json_writer = cucumber::writer::Json::new(std::io::BufWriter::new(json_file))
        .normalized::<FingerWorld>()
        .discard_stats_writes()
        .discard_arbitrary_writes();

    FingerWorld::cucumber::<&str>()
        .with_writer(
            cucumber::writer::Basic::stdout()
                .summarized()
                .tee(json_writer),
        )
        .with_default_cli()
        .repeat_skipped()
        .fail_on_skipped()
        .after(|_feature, _rule, _scenario, _finished, world| {
            Box::pin(async move {
                if let Some(world) = world {
                    support::shutdown(world);
                }
            })
        })
        .filter_run_and_exit(
            "features",
            move |feature: &gherkin::Feature,
                  rule: Option<&gherkin::Rule>,
                  scenario: &gherkin::Scenario| {
                let tags_ok = tag_expr.as_ref().is_none_or(|op| {
                    op.eval(
                        feature
                            .tags
                            .iter()
                            .chain(rule.into_iter().flat_map(|r| &r.tags))
                            .chain(&scenario.tags),
                    )
                });
                let name_ok = name_in_closure
                    .as_ref()
                    .is_none_or(|f| f.matches(&scenario.name));
                let selected = tags_ok && name_ok;
                if selected {
                    MATCHED_SCENARIOS.fetch_add(1, Ordering::SeqCst);
                }
                selected
            },
        )
        .await;

    // Zero-scenario guard: a focused run that selects nothing must fail
    // loudly instead of exiting 0 with an empty summary.
    if let Some(filter) = &name_filter
        && MATCHED_SCENARIOS.load(Ordering::SeqCst) == 0
    {
        eprintln!(
            "focused run selected 0 scenarios: --name {:?} matches no scenario \
             (regex and exact-literal fallback both miss); refusing to pass",
            filter.raw,
        );
        std::process::exit(2);
    }
}

/// The `CUCUMBER_FILTER_TAGS` tag expression, if set and parseable.
fn tag_expression_from_env() -> Option<gherkin::tagexpr::TagOperation> {
    let raw = std::env::var("CUCUMBER_FILTER_TAGS").ok()?;
    raw.parse::<gherkin::tagexpr::TagOperation>().ok()
}

/// Scenario-name filter: regex when the pattern is a valid regex matching
/// at least one known scenario, otherwise exact literal match (covers
/// names containing regex metacharacters, e.g. "finger(1) fields").
#[derive(Clone, Debug)]
struct NameFilter {
    matcher: NameMatcher,
    raw: String,
}

#[derive(Clone, Debug)]
enum NameMatcher {
    Regex(cucumber::codegen::Regex),
    Literal(String),
}

impl NameFilter {
    fn matches(&self, scenario_name: &str) -> bool {
        match &self.matcher {
            NameMatcher::Regex(re) => re.is_match(scenario_name),
            NameMatcher::Literal(lit) => scenario_name == lit,
        }
    }
}

/// Extracts the `--name <value>` / `--name=<value>` (alias
/// `--scenario-name`) argument from process argv.
fn name_arg_from_args() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2)
        .find_map(|w| {
            let (arg, next) = (&w[0], &w[1]);
            if arg == "--name" || arg == "--scenario-name" {
                Some(next.clone())
            } else {
                None
            }
        })
        .or_else(|| {
            args.iter().find_map(|arg| {
                arg.strip_prefix("--name=")
                    .or_else(|| arg.strip_prefix("--scenario-name="))
                    .map(str::to_string)
            })
        })
}

/// Builds the name filter by matching the requested pattern against every
/// scenario name found in `features/`. Regex wins when it matches
/// something; otherwise the raw string is tried as an exact scenario
/// name; when neither matches, returns `None`-matched filter so the
/// zero-scenario guard fires.
fn name_filter_from_args() -> Option<NameFilter> {
    let raw = name_arg_from_args()?;
    let names = all_scenario_names("features");
    let regex_match = cucumber::codegen::Regex::new(&raw)
        .ok()
        .filter(|re| names.iter().any(|n| re.is_match(n)));
    if let Some(re) = regex_match {
        return Some(NameFilter {
            matcher: NameMatcher::Regex(re),
            raw,
        });
    }
    if names.contains(&raw) {
        return Some(NameFilter {
            matcher: NameMatcher::Literal(raw.clone()),
            raw,
        });
    }
    // Matches nothing under either interpretation: keep the raw value for
    // the guard's error message; the filter rejects everything.
    Some(NameFilter {
        matcher: NameMatcher::Literal(String::from("\u{0}never-matches")),
        raw,
    })
}

/// Collects every scenario name from the `.feature` files under `dir`
/// (including scenarios inside rules). Best effort: unparsable files
/// contribute nothing — the guard then catches the empty selection.
fn all_scenario_names(dir: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(path) = stack.pop() {
        let entries = match std::fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map(|e| e == "feature").unwrap_or(false) {
                let env = gherkin::GherkinEnv::default();
                if let Ok(feature) = gherkin::Feature::parse_path(&path, env) {
                    for scenario in &feature.scenarios {
                        names.push(scenario.name.clone());
                    }
                    for rule in &feature.rules {
                        for scenario in &rule.scenarios {
                            names.push(scenario.name.clone());
                        }
                    }
                }
            }
        }
    }
    names
}
