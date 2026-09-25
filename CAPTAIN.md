# Captain Notes

> STOP. Captain's notes: non-binding. Captain writes, Captain trims. Anyone else: close this file now.

Binding behaviour lives in `.feature` specs and referenced `assets/**`. History lives in git. These notes carry only what the next cycle needs.

## Bootstrap decisions (2026-09-24)

- Stack: Rust, std-only for the server, no runtime deps. Confirmed with user after TypeScript/Zig comparison.
- Server = single-user finger, RFC 1288, port 79xx (default 7979), `--port` flag, prints bound port at startup.
- Reads live `~/.project` and `~/.plan` per query. Identity fields from the user's own /etc/passwd line.
- Exact match only on login or real name; unknown name = no-match answer (user chose exact match, not always-answer).
- `@host` forwarding refused ("Finger forwarding service denied"). `/W` accepted. Overlong query refused.
- Look and act exactly like finger(1), just on a different port. Multi-user/global option: later, not in specs yet.
- Client finger: second pass, not specced yet.

## Harbour 1 (2026-09-25) - decisions for Captain

- Coverage 0% sur main.rs = faux signal: le harnais tue le serveur par SIGKILL, le profraw LLVM ne se flush qu'a un exit propre. 3 pistes: (a) QM remplace child.kill() par terminate()+wait dans tests/support/mod.rs, (b) Crew ajoute un handler SIGTERM/SIGINT au serveur - mais ca doit etre pinne par un scenario avant (skeleton @captain pret dans features/ServerLifecycle.feature), (c) accepter le 0% et s'appuyer sur la couverture logique des 14 scenarios. Decision a prendre: promote ServerLifecycle (piste b) ou QM-only (piste a).
- 3 skeletons @captain a reviewer avec l'utilisateur: 2 methodology (HarborConformance.feature: watchbill shape, perturbation quiescence) + 1 product (ServerLifecycle.feature: arret propre SIGTERM).
- 7 patterns de steps non planks (Given/Then de fixtures et assertions) = normal, pas de couture produite derriere.
- RIGGING refit: verification = tests (features/steps n'existait pas), weather = .wake/weather.json, noms de deps corriges (cucumber/gherkin/tokio au lieu de @cucumber/cucumber), .ignore cree pour CAPTAIN.md.
- gplint no-homogenous-tags passe a off (squelettes @conformance tagges a l'identique le declenchent).
- Économie: 14 scenarios = 0.026s au total. Le cout est le build cargo (0.3-30s), pas l'execution. Aucun outlier.
- Ratio methodologie: 2 @conformance / 17 scenarios.
- Multi-user/global option et client finger: toujours en attente, decider au prochain voyage.
