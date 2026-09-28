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

- v0.3.1 RELEASED 2026-09-27: pushed (1f009dd), published, released binary verified: 0 CPU ticks over 3s idle (field-report probe; v0.3.0 burned 5.01 CPU-s), card served correctly. Release incident fixed live: stale v0.3.0 tarball globbed onto v0.3.1 assets, deleted from release, verify line then returns correct asset. Local tarballs removed. Mise pin may now move to 0.3.1 and restart.
- Voyage 5 COMPLETE 2026-09-27: idle-spin fixed (POLL_TICK 50ms sleep in WouldBlock arm, src/main.rs), commit e84d5cc, deck clean at rest. New spec scenario "An idle server burns no CPU" (<0.5 CPU-s over 2s, /proc/<pid>/stat ticks) pins it; 27/27 sweep green at deck hash a242035e.
- Throughput re-test owed by tester: expect ~0% idle, refusal responsive under load; accept latency <=50ms after idle tick (poll timeout).
- Public-face flip (field recommendation, needs operator ruling + deployment caveats): thimbl public (one card, no enumeration), fingerd stays loopback-only on 79. Caveats now in README: binary outside $HOME, bind source outside masked home, .plan/.project names. Cage unit checked into repo = backlog candidate.
- Harbour debts (next fitting-out-grade harbour): verification-conformance rule set; plank-inventory/step-usage slots (plank join currently a read); planted-red proof for the tier-tag arm of the watchbill conformance check; coverage re-measure (0% instrument).
- Backlog: client finger (second pass), multi-user/global mode, per-query identity liveness (spec first). Content catalog declined until client exists.
