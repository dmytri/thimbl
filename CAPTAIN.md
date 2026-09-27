# Captain Notes

> STOP. Captain's notes: non-binding. Captain writes, Captain trims. Anyone else: close this file now.

Binding behaviour lives in `.feature` specs and referenced `assets/**`. History lives in git. These notes carry only what the next cycle needs.

## Product decisions (stable)

- Rust, std-only + signal-hook. Single-user finger, RFC 1288, port 79xx (default 7979), `--port` flag, prints bound port.
- Live `~/.project`/`~/.plan` per query. Identity from own /etc/passwd, read once at startup (per-query identity liveness NOT specced - open question).
- Exact match on login or real name; unknown = no-match. `@host` refused. `/W` accepted. 512-byte limit. finger(1) long format.
- Robustness (voyage 2): 10s read timeout, EMFILE backoff + spare-fd BUSY refusal, per-connection stderr log, SIGTERM clean exit.
- Multi-user/global option: later. Client finger: second pass. Neither specced.

## State after harbour 2 + voyages 3 (2026-09-27)

- HEAD 4c0788e on origin/main. Harbour 2 plank corrections committed (4ee4f2e). Voyage 3 harness teardown committed (4c0788e): terminate() SIGTERM-first + 2s grace in tests/support.
- KEY FINDING: the 193.9s weather did NOT reproduce. Coverage-instrumented runs serialise scenarios; plain broad wall was ~11s before AND after the fix. cucumber JSON per-scenario spans at default concurrency are queue-contaminated (spans rotate between runs). Only concurrency-1 runs give a true wall prior. terminate() kept anyway: correct bounded cleanup, SIGKILL only on expiry.
- Deck clean, origin in sync. watchbill.json still present with 2 conformance targets: spent per QM (both green in broad), left unstruck per policy - Captain may withdraw or leave for next custody to strike.
- Still owed at next fitting-out-grade harbour: verification-conformance rule set (plank-form + plank-coverage token-search rules); plank-inventory/step-usage slots still none.
- Outbound candidate: v0.2.0 (robustness + planks + teardown). Ship line ready in RIGGING.md ## Outbound.

## Open questions for the user

- Release v0.2.0? (offered)
- Content catalog for 8 product-facing strings: declined for now (single consumer), revisit with the client.
- Client finger and multi-user mode: backlogged, no specs.
