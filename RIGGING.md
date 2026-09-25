# Rigging
## Stack
- language: Rust
- runtime: none
- packageManager: cargo
## Directories
- implementation: src
- specs: features
- verification: tests
- assets: none
- scantlings: features/scantlings
## Commands
- discover: none
- focused: `ref="{scenario}"; CUCUMBER_FILTER_TAGS="not @captain and not @shipwright" cargo test --test cucumber -- --name "^${ref#*:}$"`
- broad: `CUCUMBER_FILTER_TAGS="not @captain and not @shipwright" cargo test --test cucumber`
- coverage: `CUCUMBER_FILTER_TAGS="not @captain and not @shipwright" cargo llvm-cov --test cucumber`
- step-usage: none
- plank-inventory: none
- typecheck: `cargo check`
- lint: `cargo clippy --all-targets --quiet && cargo fmt --check`
- conformance: none
## Perturbation
- message: `PERTURBATION: consider current durable context; remove when fixed`
- perturb: `panic!("PERTURBATION: consider current durable context; remove when fixed");`
## Tiers
- default: @logic
- sandbox: none
- policy: default tier runs locally, no credentials
- weather: .wake/weather.json
- runrecord: .wake/runrecord.jsonl
## Dependencies
- policy: locked
- dependency: cucumber
- dependency: gherkin
- dependency: tokio
- dependency: cargo-llvm-cov
- dependency: gplint
## Outbound
- outbound: github-push
- ship: `git push origin main`
- verify: `gh api repos/dmytri/thimbl/commits/main -q .sha`
