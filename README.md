# thimbl

A single-user finger server (RFC 1288). It serves exactly one card: the
identity of the user running it, taken from its own `/etc/passwd` line,
plus the project and plan text from one state directory. Identity is
read once at startup; project and plan are re-read on every query, so
edits show up immediately.

## Running

```
thimbl --port 7979
```

The server binds `127.0.0.1` only. It is published by a TCP tunnel that
forwards a public port to the loopback port. `--port 0` binds an
ephemeral port and prints `listening on <addr>`, which is what the test
harness uses.

## Card files: the state directory

The project and plan files live in one place:

```
$HOME/.local/share/thimbl/.project
$HOME/.local/share/thimbl/.plan
```

The first run seeds the state directory: if it holds no files, thimbl
copies an existing `~/.project` and `~/.plan` into it. After that,
thimbl reads nothing else from the home; edit the files in the state
directory directly.

To migrate by hand instead:

```sh
mkdir -p ~/.local/share/thimbl
mv ~/.project ~/.plan ~/.local/share/thimbl/
```

## Isolation

thimbl serves loopback and reads nothing outside the state directory.
Keep the cage outside the process, where your supervisor already is; the
goal of the state directory is that the cage needs exactly two lines:
hide the home, expose the state directory read-only.

systemd unit:

```ini
[Service]
ExecStart=/usr/local/bin/thimbl --port 7979
TemporaryFileSystem=%h:ro
BindReadOnlyPaths=%h/.local/share/thimbl:%h/.local/share/thimbl
NoNewPrivileges=yes
```

Any other supervisor: the same cage with bubblewrap:

```sh
bwrap --unshare-all --share-net \
  --ro-bind / / \
  --tmpfs "$HOME" \
  --ro-bind "$HOME/.local/share/thimbl" "$HOME/.local/share/thimbl" \
  -- thimbl --port 7979
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
