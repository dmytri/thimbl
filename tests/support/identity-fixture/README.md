Identity fixture for the thimbl cucumber suite.

The suite's identity scenarios state their preconditions against the
invoking user's real /etc/passwd line: the card resolves the passwd
comment field, then the login, and the response steps compare both
against the identity the harness reads from the same line. /etc/passwd
is outside any fixture the harness may write, so the precondition is
provisioned by the launcher this suite runs under:

    tests/support/identity-fixture/nsrun.py

The launcher unshares a user namespace and a mount namespace, maps the
namespace uid 61000 onto the invoking user's own uid, bind-mounts an
/etc/passwd copy whose appended probe line is

    qmx999:x:61000:61000:<comment>:/home/qmx999:/bin/qmsh

and execs the suite inside, with USER and LOGNAME naming qmx999. The
comment (GECOS) field is the fixture variant, set through
THIMBL_IDENTITY_GECOS; the default is "QM GECOS Probe".

Two variants serve the card-display tier:

* plain comment "QM GECOS Probe": every untagged scenario, and the two
  scenarios asserting a non-empty comment field;
* empty comment: the two scenarios carrying @empty-comment-fixture in
  features/FingerCardDisplay.feature, whose given step "the user's
  passwd comment is empty" asserts the empty comment field.

tests/support/identity-fixture/run.sh runs the tier at a variant:

    tests/support/identity-fixture/run.sh
        # plain variant, default-tier sweep; excludes
        # @empty-comment-fixture, which reddens under the populated
        # comment
    tests/support/identity-fixture/run.sh '^<Scenario Name>$'
        # focused run, plain variant
    THIMBL_IDENTITY_GECOS= tests/support/identity-fixture/run.sh \
        'An empty comment field falls back to the login'
        # empty variant, focused
    THIMBL_IDENTITY_GECOS= tests/support/identity-fixture/run.sh \
        '@empty-comment-fixture$'
        # empty variant, the two tagged scenarios

The RIGGING.md focused command composes with the runner when the empty
variant is directed; run.sh NAME-PATTERN is its composition.
