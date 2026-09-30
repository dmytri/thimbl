# Captain Notes

> STOP. Captain's notes: non-binding. Captain writes, Captain trims. Anyone else: close this file now.

Binding behaviour lives in `.feature` specs and referenced `assets/**`. History lives in git. These notes carry only what the next cycle needs.

## Product decisions (stable)

- Rust, std-only + signal-hook. Single-user finger, RFC 1288, port 79xx (default 7979), `--port` flag, prints bound port.
- Card files live ONLY in `$HOME/.local/share/thimbl/` (`.plan`, `.project`), read live per query. No `--state-dir` flag, no XDG honouring: fixed path keeps the cage expose-set a constant; HOME already relocates per-user (harness uses temp HOME). Ruling 2026-09-27.
- Seed on first run: empty state dir => copy `~/.project`/`~/.plan` in, once, before the listening socket binds. Seed failure seeds nothing (default card). Ruling 2026-09-27.
- Isolation stays outside the process: systemd unit (`TemporaryFileSystem=%h:ro` + one BindReadOnlyPaths line) or bwrap one-liner; no in-process jail this voyage. In-process jail declined 2026-09-27 (riskiest code in repo protecting deployments using no cage); revisit only if bare deployments become real. If ever built: sandbox-when-possible posture, rustix+seccompiler, macOS = no sandbox, default card = today's display.
- Live `~/.project`/`~/.plan` per query: SUPERSEDED by state dir. Identity from own /etc/passwd, read once at startup.
- Exact match on login or real name; unknown = no-match. `@host` refused. `/W` accepted. 512-byte limit. finger(1) long format.
- Robustness (voyage 2): 10s read timeout, EMFILE backoff + spare-fd BUSY refusal, per-connection stderr log, SIGTERM clean exit.
- Multi-user/global option: later. Client finger: second pass. Neither specced.

## Released

- v0.2.0 (2026-09-27): robustness suite + plank corrections + harness teardown. Tag pushed, tarball verified live (downloaded, checksum match, binary ran and answered). HEAD 81692d3.

## Measurement caveats (verified 2026-09-27)

- The 193.9s weather did NOT reproduce: coverage-instrumented runs serialise scenarios. Plain broad wall ~11-13s.
- cucumber JSON per-scenario spans at default concurrency (64) are queue-contaminated: the ~10s span rotates between scenarios across runs. Only concurrency-1 runs give a true wall prior.
- Idle-reap scenario legitimately takes ~10s: it observes the product's 10s reaping window. Pinned behaviour, correct to be slow.

## State and next

- v0.5.0 RELEASED 2026-09-29 (153b6b8, verified live). Card format: one "User:" line resolved .user > GECOS > login; units need `thimbl serve --port N` + init before pin move (operator-side; tunnel currently serves fingerd).
- QM watchbill incident (voyage 6): QM struck a valid tier-tag watch once; tier-tag arm of the watchbill conformance check still owes its planted-red proof at next promotion.
- Throughput re-test owed by tester: expect ~0% idle, refusal responsive under load, accept <=50ms after idle tick.
- Public-face flip (field recommendation, needs operator ruling + deployment caveats): thimbl public (one card, no enumeration), fingerd stays loopback-only on 79. Cage unit checked into repo = backlog candidate.
- Backlog: client finger (second pass), multi-user/global mode, per-query identity liveness (spec first). Content catalog declined until client exists.

## Voyage 10 COMPLETE (2026-09-30): methodology conformance round (654efef)

 Origin of the round: harbour review converted Shipwright findings into binding conformance scenarios (operator questioned the conversion mid-flight; ruled after the fact with "I accept"). Three targets landed: plank-join extended with @planks-provisional clause; inverse join "Every behaviour-bearing step pattern carries a plank"; "The coverage run measures the implementation" (coverage-scoped nested llvm-cov run, excludes @coverage-self). QM fixed the harness coverage defect (server_binary honours llvm-cov-target dir + THIMBL_BIN, child LLVM_PROFILE_FILE, dropped --no-clean): measured src coverage 88.55% (was 0.00%). Crew mate corrected 15 plank-form docblocks in src/main.rs (plank-only diff, 0 behaviour lines). Proofs: 59 scenarios/313 steps green sweep, planted-red x3, custody commit 654efef, tree clean. Watchbill struck. Next coverage-fixed run should refresh the stale weather record.

 Harbour finding (Boatswain, non-blocking, for next fitting-out harbour): 5 orphan step definitions in tests/cucumber/definitions.rs — 2 lost usages at d72d4b1 (retired card format), 3 never lived in any features tree ("contains a {string} section with the current plan", "contains a {string} line with the current user", "it exits with the code {int}"). Removal is Shipwright's. Also: RIGGING has no step-usage command (sanctioned fallback census used); a derived step-usage command would make that check runnable.

 Captain rulings this session: Shipwright cluster findings (3-seam stdio planks, 3-seam port-0, 2-seam SIGTERM/identity-line, duplicated overlong-reply literal) are acceptable multi-carrier planks, no consolidation perturbation ordered; transitive dep updates (serde_with, smallvec) stay locked; weather refresh deferred to next coverage-fixed run (stale 21-scenario record from 2026-09-27 known void).

 Voyage 9 COMPLETE: stdio serve landed (033f8a4, custody ahead of origin by 1, push pending operator). 6/6 FingerStdio green, 57/305 sweep green. Prior notes absorbed: Voyage 8 (identity collapse), Voyage 6 (init/serve split), v0.5.0 released, Harbour 3.

 Still open from prior cycles: throughput re-test owed by tester (idle ~0%, refusal under load, <=50ms post-tick); public-face flip needs operator ruling; backlog: client finger, multi-user, per-query identity liveness.
