> STOP. Captain's notes: non-binding. Captain writes, Captain trims. Anyone else: close this file now.

# Captain Notes

Binding behaviour lives in `.feature` specs and referenced `assets/**`. History lives in git. These notes carry only what the next cycle needs.

## Bootstrap decisions (2026-09-24)

- Stack: Rust, std-only for the server, no runtime deps. Confirmed with user after TypeScript/Zig comparison.
- Server = single-user finger, RFC 1288, port 79xx (default 7979), `--port` flag, prints bound port at startup.
- Reads live `~/.project` and `~/.plan` per query. Identity fields from the user's own /etc/passwd line.
- Exact match only on login or real name; unknown name = no-match answer (user chose exact match, not always-answer).
- `@host` forwarding refused ("Finger forwarding service denied"). `/W` accepted. Overlong query refused.
- Look and act exactly like finger(1), just on a different port. Multi-user/global option: later, not in specs yet.
- Client finger: second pass, not specced yet.

## Open items

- cargo-llvm-cov installed during bootstrap; runner composition (`focused` command shape) to be confirmed at QM/Crew first run.
- Plank-inventory: none at bootstrap; bespoke checker lands at first harbour via QM route.
- Verification harness (tests/cucumber harness, step definitions) is QM's write scope, not yet created.
