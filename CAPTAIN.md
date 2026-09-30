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

- v0.5.0 RELEASED 2026-09-29: pushed (153b6b8), published with explicit asset name, verify line + asset list clean. Released artifact smoke-proven: init seeds .user + links dot-files, served card shows "User: Admiral Dmytri" from .user. Card-format break live: "Login name:/In real life:/Directory:/Shell:" gone, one User line; units need `thimbl serve --port N` + init before pin move (operator-side; tunnel currently serves fingerd).

- Voyage 8 COMPLETE 2026-09-27: identity collapsed to one "User:" line (d72d4b1). Resolution: .user (state dir, per-query) > GECOS comment (first comma segment) > login; empty falls through. init seeds .user from GECOS/login, never overwrites, no home symlink; "exactly like finger(1)" promise retired. Identity fixture landed in-tree: tests/support/identity-fixture/run.sh (plain variant = default sweep, excludes @empty-comment-fixture; THIMBL_IDENTITY_GECOS= selects empty variant; proven 47/262 + 2/2). @empty-comment-fixture tier decision OPTION 2 landed (support excludes; no RIGGING tier entry). v0.5.0 pending release call (card-format break). Ahead 1.
- Voyage 6 COMPLETE 2026-09-27: thimbl init + serve split landed (db52d58 init/serve/no-autoseed, d612bfe output reporting). 39 scenarios/215+ steps sweep green twice (hash 374c6c1b), init-output focused greens (hash b768df3f). Init smoke-proven on six shapes (fresh/second/import/symlink-elsewhere/force/no-link).
- QM watchbill incident (voyage 6 round 2): QM rejected valid `@logic` tier-tag watch as malformed shell token and struck the file; Captain restored + re-struck through proper custody. Watchbill validation bug = candidate for conformance rule set at harbour (tier-tag arm already owed planted-red proof).
- Throughput re-test owed by tester: expect ~0% idle, refusal responsive under load; accept latency <=50ms after idle tick (poll timeout).
- Public-face flip (field recommendation, needs operator ruling + deployment caveats): thimbl public (one card, no enumeration), fingerd stays loopback-only on 79. Caveats now in README: binary outside $HOME, bind source outside masked home, .plan/.project names. Cage unit checked into repo = backlog candidate.
- Harbour debts (next fitting-out-grade harbour): verification-conformance rule set; plank-inventory/step-usage slots (plank join currently a read); planted-red proof for the tier-tag arm of the watchbill conformance check; coverage re-measure (0% instrument).
- Backlog: client finger (second pass), multi-user/global mode, per-query identity liveness (spec first). Content catalog declined until client exists.

## Harbour 3 (2026-09-30)

 Shipwright ran (base fe27fbd): rigging refit (broad/coverage now exclude @empty-comment-fixture; lint chains gplint first; plank-inventory = rg token search), 4 seams planked (usage, seed_user_file, Finger::identity, Finger::user_file_identity), 47/47 tier green, real coverage measured once at 86.8% lines.

 Operator rulings this session: promote all 3 CLI refusal scenarios (exit 2 pinned), promote plank-form conformance scenario, remove the 4 orphaned step definitions (2 are retired card-format leftovers).

 Watchbill carries 3 watches: focused CLI trio (proven green once, focused greens at current hash owe QM), plank-form scenario (steps undefined, QM authors), then @logic sweep (proves the whole deck post-refit).

 Standing blockers for QM, from Shipwright evidence: coverage reports 0% because server_binary() spawns target/<profile>/thimbl, uninstrumented; clean fix = honour CARGO_LLVM_COV_TARGET_DIR or a THIMBL_BIN override, then simplify the coverage command. A verification-conformance rule set is still absent (no engine derivable; conformance: none).

## Voyage 9 (2026-09-30): serve --stdio

 Operator ruled: `thimbl serve --stdio` serves the finger protocol over stdin/stdout, one query per process, exit 0. Per-connection socket activation (inetd / systemd socket unit Accept=yes StandardInput=socket); NOT fd-passing LISTEN_FDS. Specs in features/FingerStdio.feature (6 scenarios), watchbill watch3, focused red confirmed: 6 scenarios 6 failed, "Step doesn't match any function" (12 of 18 steps bind existing patterns: identity line, project/plan content, no-match, forwarding refusal, query-too-long, exit code, "does not contain"). Startup-line regression pinned: stdio stdout must never carry "listening on". Usage error must stay on stderr (unpinned explicitly; QM should keep stdout = protocol only).
