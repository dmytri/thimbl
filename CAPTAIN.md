# Captain Notes

> STOP. Captain's notes: non-binding. Captain writes, Captain trims. Anyone else: close this file now.

Binding behaviour lives in `.feature` specs and referenced `assets/**`. History lives in git. These notes carry only what the next cycle needs.

## Product decisions (stable)

- Rust, std-only + signal-hook. Single-user finger, RFC 1288, port 79xx (default 7979), `--port` flag, prints bound port.
- Live `~/.project`/`~/.plan` per query. Identity from own /etc/passwd, read once at startup (per-query identity liveness NOT specced - open question).
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

- Deck clean, ahead of origin/main by 1 (9f33e93 watchbill strike). Prior "struck at v0.2.0 custody prep" was wrong: watchbill.json stayed tracked and present from voyage 3 (4c0788e) through v0.2.0. Struck 2026-09-27 at voyage 4 open, post-reset, after confirming spent (hand-off report + runrecord pass at the v0.2.0 deck state).
- Still owed at next fitting-out-grade harbour: verification-conformance rule set (plank-form + plank-coverage token-search rules); plank-inventory/step-usage slots still none. Coverage instrument still reads 0% (child SIGKILL in some paths) - SIGTERM-first teardown improves it; re-measure next harbour.
- Backlog: client finger (second pass), multi-user/global mode, per-query identity liveness (spec first). Content catalog declined until client exists.
- Session reset consumed 2026-09-27: this fresh context is the post-reset cycle.
