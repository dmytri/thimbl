# thimbl

A single-user finger server (RFC 1288). It serves exactly one card: an
identity line plus the project and plan text from one state directory.
The identity line resolves in this order: the state directory's `.user`
file, the comment field of the user's own `/etc/passwd` line, the login
name; an empty value falls through to the next source. Project, plan
and user files are re-read on every query, so edits show up
immediately.

## Serving

```
thimbl serve --port 7979
```

`thimbl serve` is the only serving entry point; bare `thimbl` prints
usage and exits non-zero. `--port 0` binds an ephemeral port and prints
`listening on <addr>`, which is what the test harness uses.

The server binds `127.0.0.1` only. It is published by a TCP tunnel that
forwards a public port to the loopback port.

## Card files: the state directory

The project and plan files live in one place:

```
$HOME/.local/share/thimbl/.project
$HOME/.local/share/thimbl/.plan
```

Set the directory up once, before enabling any supervisor:

```sh
thimbl init
```

`thimbl init` imports your classic `~/.project` / `~/.plan` into the
state directory (never overwriting existing state files; a differing
home file is reported as a conflict and left untouched unless
`thimbl init --force` moves its content in), then points your home
dot-files at the state files as relative symlinks. Editor and
`finger`-adjacent muscle memory keep working; writes through the link
land in the state directory and are served live. Run it twice and the
second run reports `kept` and changes nothing. `--no-link` does the
state-directory work only, for deployments that keep the real files
elsewhere and bind them onto the fixed path (a system unit binding
from `/srv` populates the bind source directly; `init` cannot see
inside the cage, and a caged start over an empty state directory
publishes the default card). Note that an empty state file serves a
blank section, exactly like finger(1); only a missing file serves
`No Plan.` / `No Project.`.

`thimbl init` never binds a socket, and `serve` does no setup work:
the split keeps setup runnable outside any cage, since a supervisor's
sandboxing applies to its own pre-start hooks too.

## Isolation

thimbl serves loopback and reads nothing outside the state directory.
Keep the cage outside the process, where your supervisor already is; the
goal of the state directory is that the cage needs exactly two lines:
hide the home, expose the state directory read-only.

systemd unit:

```ini
[Service]
ExecStart=/usr/local/bin/thimbl serve --port 7979
TemporaryFileSystem=%h:ro
BindReadOnlyPaths=%h/.local/share/thimbl:%h/.local/share/thimbl
NoNewPrivileges=yes
```

The `BindReadOnlyPaths` form works as written in a **user** unit
(systemd resolves the source inside `%h` before masking); a **system**
unit cannot bind from inside the masked home (it fails with
`226/NAMESPACE`), so there the real files live outside `$HOME` (for
example `/srv/thimbl`) and are bound onto the fixed path.

Any other supervisor: the same cage with bubblewrap. bwrap validates
bind sources before applying mounts, so nothing may be sourced from
inside the masked home; the card files live outside `$HOME` (here
`$XDG_RUNTIME_DIR/thimbl`, created by `thimbl init --no-link` against
that home or filled directly) and the binary is re-bound into a tmpfs:

```sh
bwrap --unshare-all --share-net \
  --ro-bind / / --dev /dev --proc /proc \
  --tmpfs /tmp --ro-bind /usr/local/bin/thimbl /tmp/thimbl \
  --tmpfs "$HOME" \
  --ro-bind "$XDG_RUNTIME_DIR/thimbl" "$HOME/.local/share/thimbl" \
  -- /tmp/thimbl serve --port 7979
```

The bubblewrap form needs unprivileged user namespaces enabled in the
kernel. Run thimbl under whichever supervisor you prefer; both forms
give the same view: the server sees its state directory and nothing
else of the home.

Two placement rules make the cage work:

- Install the binary outside `$HOME` (for example
  `/usr/local/bin/thimbl`): a masked home makes any binary that lives
  under it unreachable to the supervisor.
- The bind source must exist before the cage starts. A system unit
  cannot bind a path that lives under the masked home; keep the real
  state directory outside `$HOME` (for example `/srv/thimbl`) and bind
  it onto the fixed path. In a caged deployment the fixed path is a
  mount point; the files live wherever the bind source lives.

The state files must be named `.plan` and `.project` even when the
state directory itself lives elsewhere.

## Verifying the cage

With the server running under the unit or the bubblewrap line:

```sh
pid=$(pgrep -n thimbl)
grep NoNewPrivs /proc/$pid/status
grep -E 'tmpfs|.local/share/thimbl' /proc/$pid/mountinfo
```

`NoNewPrivs: 1` and a mount table showing a tmpfs over the home plus the
read-only state directory bind are the claim. If the lines are missing,
the cage is not on; fix the supervisor configuration before publishing.

## Built with Shipshape

This repository uses [Shipshape](https://github.com/dmytri/shipshape), a context-isolated spec-driven workflow for coding agents. Install with `npx skills add dmytri/shipshape --skill '*'`, or the experimental open-plugin build with `npx plugins add dmytri/shipshape`.
