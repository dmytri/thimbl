# Rigging
## Stack
- language: Rust
- runtime: none
- packageManager: cargo
## Directories
- implementation: src
- specs: features
- verification: features/steps
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
- weather: none
- runrecord: .wake/runrecord.jsonl
## Dependencies
- policy: locked
- dependency: @cucumber/cucumber
- dependency: gherkin
- dependency: cargo-llvm-cov
- dependency: gplint
## Outbound
- outbound: none
