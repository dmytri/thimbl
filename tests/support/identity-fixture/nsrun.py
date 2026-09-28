#!/usr/bin/env python3
"""Identity fixture launcher for the thimbl cucumber suite.

The scenarios that state identity preconditions (the passwd comment
field, the login) run against the invoking user's real /etc/passwd line,
which no fixture the harness may write can change. This launcher
provisions the precondition instead:

* unshares a user namespace and a mount namespace,
* maps the namespace uid 61000 onto the invoking user's own uid,
* bind-mounts an /etc/passwd copy whose appended probe line carries the
  probe login qmx999,
* execs argv[1:] inside, with USER and LOGNAME naming the probe login.

The probe line's comment (GECOS) field selects the fixture variant:
THIMBL_IDENTITY_GECOS overrides the default "QM GECOS Probe". An empty
value serves the two @empty-comment-fixture scenarios in
features/FingerCardDisplay.feature, whose given step
"the user's passwd comment is empty" asserts the empty comment field.
The value must not contain ":".

Invocation:

    tests/support/identity-fixture/run.sh           # default tier sweep
    tests/support/identity-fixture/run.sh '^Name$'  # focused run
"""

import ctypes
import os
import shutil
import sys
import tempfile

CLONE_NEWNS = 0x00020000
CLONE_NEWUSER = 0x10000000
MS_BIND = 4096

PROBE_LOGIN = "qmx999"
PROBE_UID = 61000
DEFAULT_GECOS = "QM GECOS Probe"


def die(message):
    print(f"identity-fixture: {message}", file=sys.stderr)
    sys.exit(1)


def main():
    if len(sys.argv) < 2:
        die("usage: nsrun.py COMMAND [ARGS...]")
    libc = ctypes.CDLL(None, use_errno=True)

    uid, gid = os.getuid(), os.getgid()
    if libc.unshare(CLONE_NEWUSER | CLONE_NEWNS) != 0:
        die(f"unshare: {os.strerror(ctypes.get_errno())}")

    # One outer id is available, so the map carries one entry: namespace
    # uid 61000 is the invoking user; every other namespace id stays
    # unmapped and reads as nobody.
    with open("/proc/self/uid_map", "w") as f:
        f.write(f"{PROBE_UID} {uid} 1\n")
    with open("/proc/self/setgroups", "w") as f:
        f.write("deny\n")
    with open("/proc/self/gid_map", "w") as f:
        f.write(f"{PROBE_UID} {gid} 1\n")

    gecos = os.environ.get("THIMBL_IDENTITY_GECOS", DEFAULT_GECOS)
    if ":" in gecos:
        die("THIMBL_IDENTITY_GECOS must not contain ':'")

    with tempfile.TemporaryDirectory(prefix="qm-identity-") as tmpdir:
        shadow = os.path.join(tmpdir, "passwd")
        shutil.copyfile("/etc/passwd", shadow)
        with open(shadow, "a") as f:
            f.write(f"\n{PROBE_LOGIN}:x:{PROBE_UID}:{PROBE_UID}:{gecos}:"
                    f"/home/{PROBE_LOGIN}:/bin/qmsh\n")
        if libc.mount(shadow.encode(), b"/etc/passwd", None, MS_BIND, None) != 0:
            die(f"mount /etc/passwd: {os.strerror(ctypes.get_errno())}")

        os.environ["USER"] = PROBE_LOGIN
        os.environ["LOGNAME"] = PROBE_LOGIN
        os.execvp(sys.argv[1], sys.argv[1:])
        die(f"execvp {sys.argv[1]} failed")


if __name__ == "__main__":
    main()
