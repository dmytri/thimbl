# Captain Notes

> STOP. Captain's notes: non-binding. Captain writes, Captain trims. Anyone else: close this file now.

Binding behaviour lives in `.feature` specs and referenced `assets/**`. History lives in git. These notes carry only what the next cycle needs.

## Bootstrap decisions (2026-09-24)

- Stack: Rust, std-only server (signal-hook added voyage 2 for SIGTERM). Single-user finger, RFC 1288, port 79xx (default 7979), `--port` flag, prints bound port at startup.
- Reads live `~/.project` and `~/.plan` per query. Identity from the user's own /etc/passwd line, read once at startup (restart shows GECOS edits; per-query identity liveness NOT specced - open question).
- Exact match on login or real name; unknown = no-match. `@host` refused. `/W` accepted. 512-byte limit. finger(1) long format.
- Multi-user/global option: later. Client finger: second pass. Neither specced.

## Harbour 2 (2026-09-25) - state at Captain review

- Shipwright full regression: 21/21 green, 119 steps. Plank corrections in flight (uncommitted, src/main.rs): 2 malformed planks fixed (scenario-name planks on serve() and refuse_one_backlogged() replaced with real step patterns). Joint now 34 planks / 0 unresolved / 0 provisional.
- Shipwright findings awaiting Captain routing:
  1. HARNESS TEARDOWN DEBT (highest leverage): server 10s READ_TIMEOUT leaks into After hook; 18/21 scenarios sit at ~10.2s floor; total 193.9s. Fix is QM-scope (SIGTERM-first-then-kill in tests/support). Not a product defect.
  2. Content catalog: 8 product-facing strings in main.rs; assets: none today. Captain decision.
  3. Coverage 0% instrument artifact: same SIGKILL-vs-clean-exit cause; the QM teardown fix (route 1) also repairs it.
  4. Verification-conformance rule set (plank rules) still unwritten - owed at next fitting-out-grade harbour.
  5. 3 transitive deps behind (serde_with, smallvec) - locked policy, no action.
- Deck: src/main.rs modified (plank fixes only, no behaviour change). Needs Boatswain custody.

## Decisions this session (2026-09-25, post-harbour)

- Teardown debt: route to QM as a watchbill target? NOT YET DECIDED - pending user word.
- Outbound: v0.2.0 release candidate (robustness + plank fixes) - offered, pending user word.
