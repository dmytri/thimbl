//! thimbl — a single-user RFC 1288 finger server.
//!
//! The command surface is explicit: `thimbl init` establishes the card
//! state directory, `thimbl serve` serves it, and bare `thimbl` prints
//! usage and exits non-zero.
//!
//! Serving answers with exactly one card: one identity line resolving
//! the user's name from the state directory's `.user` file, then the
//! user's own `/etc/passwd` comment field, then the login, plus the
//! `.project` and `.plan` card files in the state directory
//! (`$HOME/.local/share/thimbl`). The card files are re-read from the
//! state directory on every query, so edits show up immediately. The
//! server itself does no setup work: establishment is `init`'s job, so
//! setup stays runnable outside any supervisor sandbox.
//!
//! Over TCP the server answers the RFC 1288 `{C}` query grammar:
//!
//! * an empty query — or `/W` alone, the verbose switch, which is
//!   accepted and ignored — returns the full card in finger(1) long
//!   format;
//! * a query naming the login or the real name returns the same card;
//! * a query containing `@` is refused: forwarding is not offered;
//! * a query longer than [`MAX_QUERY_BYTES`] is refused;
//! * anything else is answered with a no-such-user notice.
//!
//! Every response line ends with CRLF and the connection is closed after
//! the answer. The bound address is printed to stdout at startup so a
//! supervisor can learn which port was chosen (`--port 0` asks the
//! kernel for a free one, which is the default).

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::thread;
use std::time::Duration;

/// Largest accepted query line, in bytes, excluding the CRLF terminator.
const MAX_QUERY_BYTES: usize = 512;

/// RFC 1288 line terminator used for every response line.
const CRLF: &str = "\r\n";

/// Sent to a client accepted from the backlog while the server is out of
/// file descriptors; the connection is closed right after.
const BUSY_REPLY: &str = "finger: server busy, try again\r\n";

/// Idle connections are reaped: a client that sends nothing within this
/// window finds its connection closed by the server.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// How long the accept loop sleeps between non-blocking accept retries
/// when no client is waiting. Keeps the loop's SIGTERM checks well inside
/// the one-second clean-exit bound while an idle server uses no CPU.
const POLL_TICK: Duration = Duration::from_millis(50);

/// How long the accept loop waits between retries while the process is out
/// of file descriptors. The kernel keeps queueing connecting clients in the
/// listen backlog meanwhile, so a client that arrives during exhaustion is
/// answered as soon as one descriptor frees.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

/// The server listens on loopback only; this is a single-user service.
const HOST: &str = "127.0.0.1";

/// Set by the SIGTERM handler; the accept loop observes it and shuts the
/// server down cleanly. Only an atomic store runs inside the signal handler.
static TERMINATED: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));

/// @planks("thimbl runs with no subcommand")
/// @planks("it exits non-zero")
/// @planks("the finger server is started with the serve command on port 0")
/// Dispatches the explicit command surface: `init` establishes the card
/// state directory, `serve` serves it (also the port-parse error path),
/// and bare `thimbl` or an unknown subcommand prints usage and exits
/// non-zero — serving is never implicit.
fn main() -> ExitCode {
    match env::args().nth(1).as_deref() {
        // Bare `thimbl` and anything unrecognized are refusals, not
        // servers: serving is deliberate, behind the `serve` subcommand.
        Some("--help") => {
            usage();
            ExitCode::SUCCESS
        }
        // Socket activation: `--stdio` serves exactly one query from
        // stdin to stdout — no listener, no startup line — so a
        // supervisor's per-connection unit gets its answer and a clean
        // exit. Any other serve argument keeps the port parsing.
        Some("serve") if env::args().skip(2).any(|arg| arg == "--stdio") => serve_stdio(),
        Some("serve") => match port_from_args() {
            Ok(port) => serve(port),
            Err(message) => {
                eprintln!("thimbl: {message}");
                usage();
                ExitCode::from(2)
            }
        },
        Some("init") => init_state_dir(),
        Some(unknown) => {
            eprintln!(
                "thimbl: {}{unknown:?}",
                if unknown == "--port" || unknown.starts_with("--port=") {
                    "serving needs the serve subcommand: "
                } else {
                    "unknown subcommand "
                },
            );
            usage();
            ExitCode::from(2)
        }
        None => {
            usage();
            ExitCode::from(2)
        }
    }
}

/// @planks("the output names the serve command")
/// @planks("the output names the init command")
/// Prints the command surface, naming `serve` as the serving entry point.
fn usage() {
    eprintln!("usage: thimbl init [--force] [--no-link]");
    eprintln!("       thimbl serve [--port PORT]");
}

/// @planks("thimbl init runs")
/// @planks("thimbl init runs again")
/// @planks("thimbl init runs with {string}")
/// @planks("thimbl init has run")
/// @planks("the init run exits with code {int}")
/// @planks("the init output names the state directory")
/// Establishes the card state directory from the home dot-files: each
/// card file's state directory copy is seeded from the home dot-file
/// when missing (empty when the home has none), never overwritten when
/// present; the home dot-file is then pointed at the state file as a
/// relative symlink unless `--no-link` asks otherwise. A home regular
/// file whose content differs from the state file is a conflict: it is
/// reported and left alone, or with `--force` its content is moved into
/// the state file and the link replaces it. The `.user` identity file
/// is seeded from the passwd comment and never overwritten, with no
/// home symlink.
fn init_state_dir() -> ExitCode {
    let mut force = false;
    let mut link = true;
    for arg in env::args().skip(2) {
        match arg.as_str() {
            "--force" => force = true,
            "--no-link" => link = false,
            "--help" => {
                usage();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("thimbl: unknown argument {other:?}");
                usage();
                return ExitCode::from(2);
            }
        }
    }
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        eprintln!("thimbl: HOME is not set");
        return ExitCode::FAILURE;
    };
    let state_dir = home.join(".local/share/thimbl");
    if let Err(err) = fs::create_dir_all(&state_dir) {
        eprintln!("thimbl: cannot create {}: {err}", state_dir.display());
        return ExitCode::FAILURE;
    }
    eprintln!("thimbl: state directory {}", state_dir.display());
    let mut failed = false;
    for file in [".project", ".plan"] {
        if let Err(err) = establish_card(&home, &state_dir, file, force, link) {
            eprintln!("thimbl: {err}");
            failed = true;
        }
    }
    seed_user_file(&state_dir);
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// @planks("the state directory has the user content from the passwd comment")
/// @planks("the state directory has the user content {string}")
/// Seeds the state directory's `.user` identity file from the passwd
/// comment field's first comma segment, falling back to the login, when
/// the file is missing; an existing `.user` is never overwritten. The
/// home gets no `.user` symlink: it is an identity file, not a classic
/// dot-file.
fn seed_user_file(state_dir: &std::path::Path) {
    let state_file = state_dir.join(".user");
    if state_file.exists() {
        eprintln!("thimbl: {}: kept", state_file.display());
        return;
    }
    let identity = current_identity();
    let value = if identity.real_name.is_empty() {
        &identity.login
    } else {
        &identity.real_name
    };
    if let Err(err) = fs::write(&state_file, value.as_bytes()) {
        eprintln!("thimbl: cannot seed {}: {err}", state_file.display());
    } else {
        eprintln!("thimbl: {}: seeded", state_file.display());
    }
}

/// @planks("the home has a project file with content {string}")
/// @planks("the home has a plan file with content {string}")
/// @planks("the home has no plan file")
/// @planks("the state directory has a plan file with content {string}")
/// @planks("the state directory has no plan file")
/// @planks("the state directory has the project content {string}")
/// @planks("the state directory has the plan content {string}")
/// @planks("the state directory has an empty plan file")
/// @planks("the home project links to the state file")
/// @planks("the home plan links to the state file")
/// @planks("the home plan is still a regular file")
/// @planks("the home plan is still a symlink to elsewhere")
/// @planks("the init output names a conflict")
/// @planks("the init output names the home plan")
/// @planks("the init output names the plan kept")
/// @planks("the init output names the project and the plan")
/// One card file's establishment, per the [`init_state_dir`] policy.
/// `Ok(())` when the home dot-file ends up linked (or left untouched by
/// `--no-link`/a reported conflict); `Err` only on I/O failure.
fn establish_card(
    home: &std::path::Path,
    state_dir: &std::path::Path,
    file: &str,
    force: bool,
    link: bool,
) -> Result<(), String> {
    let home_file = home.join(file);
    let state_file = state_dir.join(file);
    let home_meta = fs::symlink_metadata(&home_file);
    let home_link = home_meta
        .as_ref()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false);
    let home_bytes = if home_link {
        None
    } else {
        fs::read(&home_file).ok()
    };
    let state_bytes = fs::read(&state_file).ok();
    // The home regular file counts as a conflict only when its content
    // differs from what the state file carries; a home file whose content
    // was just seeded in (or already matched) is simply linked over.
    let mut conflict = false;
    match (home_bytes, state_bytes) {
        // Nothing in the home, nothing in the state directory: the state
        // file is created empty and the link is made.
        (None, None) => {
            fs::write(&state_file, b"")
                .map_err(|err| format!("cannot create {}: {err}", state_file.display()))?;
            eprintln!("thimbl: {}: seeded", state_file.display());
        }
        // The home has content the state directory lacks: seed it in.
        (Some(home_content), None) => {
            fs::write(&state_file, &home_content)
                .map_err(|err| format!("cannot seed {}: {err}", state_file.display()))?;
            eprintln!("thimbl: {}: seeded", state_file.display());
        }
        // The state file exists and is never overwritten. A differing
        // home regular file is a conflict: reported and left alone, or
        // with `--force` its content is kept by moving it into the state
        // file.
        (Some(home_content), Some(state_content)) => {
            if home_content != state_content {
                if force {
                    fs::write(&state_file, &home_content)
                        .map_err(|err| format!("cannot seed {}: {err}", state_file.display()))?;
                    eprintln!("thimbl: {}: replaced", state_file.display());
                } else {
                    conflict = true;
                    eprintln!(
                        "thimbl: conflict: {} differs from {}",
                        home_file.display(),
                        state_file.display(),
                    );
                }
            }
        }
        // The state file exists (whatever its content, including empty)
        // and the home has no regular file: the state file is kept and
        // named so.
        (None, Some(_)) => {
            eprintln!("thimbl: {}: kept", state_file.display());
        }
    }
    if !link {
        eprintln!("thimbl: {}: left, --no-link", home_file.display());
        return Ok(());
    }
    match home_meta {
        // Already a symlink: leave it alone whatever it points at.
        Ok(meta) if meta.file_type().is_symlink() => {
            eprintln!(
                "thimbl: {}: already a symlink, left alone",
                home_file.display()
            );
            return Ok(());
        }
        // A conflicting home regular file keeps the home file unless
        // `--force` relinks it; the report above already named it.
        Ok(_) if conflict && !force => {
            return Ok(());
        }
        // Replace any existing home entry with the link.
        Ok(_) | Err(_) => {}
    }
    let link_target = relative_link(&home_file, &state_file);
    #[cfg(unix)]
    {
        let _ = fs::remove_file(&home_file);
        std::os::unix::fs::symlink(&link_target, &home_file).map_err(|err| {
            format!(
                "cannot link {} to {}: {err}",
                home_file.display(),
                state_file.display()
            )
        })?;
        eprintln!(
            "thimbl: {}: linked to {}",
            home_file.display(),
            link_target.display()
        );
    }
    #[cfg(not(unix))]
    {
        let _ = link_target;
        return Err(format!(
            "linking {} requires a POSIX filesystem",
            home_file.display()
        ));
    }
    Ok(())
}

/// The link text that points `from` at `to`: relative when `to` sits
/// below `from`'s parent, absolute otherwise.
fn relative_link(from: &std::path::Path, to: &std::path::Path) -> PathBuf {
    let link_dir = from.parent().unwrap_or_else(|| std::path::Path::new(""));
    match to.strip_prefix(link_dir) {
        Ok(rest) => rest.to_path_buf(),
        Err(_) => to.to_path_buf(),
    }
}

/// @planks("the process receives the signal SIGTERM")
/// Arms the SIGTERM handler before any connection thread exists. The
/// handler only flips an atomic flag (the one async-signal-safe thing it
/// needs to do); the accept loop wakes from its listener timeout, sees the
/// flag, and `serve` returns a clean success exit.
fn install_sigterm_handler() -> io::Result<()> {
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&TERMINATED))?;
    Ok(())
}

/// @planks("the finger server is started on port 0")
/// Parses the optional `--port N` / `--port=N` argument. Defaults to 0 so
/// the kernel picks a free port; the resolved port is printed at startup.
fn port_from_args() -> Result<u16, String> {
    let mut args = env::args().skip(2);
    let mut port = 0;
    while let Some(arg) = args.next() {
        let raw = if arg == "--port" {
            args.next().ok_or("--port requires a value")?
        } else if let Some(raw) = arg.strip_prefix("--port=") {
            raw.to_string()
        } else if arg == "--help" {
            usage();
            std::process::exit(0);
        } else {
            return Err(format!("unknown argument {arg:?}"));
        };
        port = raw
            .parse::<u16>()
            .map_err(|err| format!("invalid --port value {raw:?}: {err}"))?;
    }
    Ok(port)
}

/// @planks("the finger server is started on port 0")
/// @planks("the process receives the signal SIGTERM")
/// @planks("the process exits within one second with code {int}")
fn serve(port: u16) -> ExitCode {
    install_sigterm_handler()
        .unwrap_or_else(|err| eprintln!("thimbl: cannot arm SIGTERM handler: {err}"));
    let finger = Arc::new(Finger::current());
    let listener = match TcpListener::bind((HOST, port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("thimbl: cannot bind {HOST}:{port}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let bound = match listener.local_addr() {
        Ok(bound) => bound,
        Err(err) => {
            eprintln!("thimbl: cannot read bound address: {err}");
            return ExitCode::FAILURE;
        }
    };
    println!("listening on {bound}");
    let _ = io::stdout().flush();
    // A non-blocking accept returns immediately with `WouldBlock` when no
    // client is waiting: the loop then revisits the SIGTERM check every
    // tick instead of blocking inside accept until the next connection.
    // The tick sleeps so an idle server does not spin a core between
    // checks.
    let _ = listener.set_nonblocking(true);
    // Reserve one descriptor up front, while the table still has room: it
    // is spent during exhaustion so a queued client can be accepted and
    // refused immediately instead of hanging until a descriptor frees.
    let mut spare = listener.try_clone().ok();
    let mut exhaust_flood = false;
    while !TERMINATED.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, peer)) => {
                // The accepted stream inherits O_NONBLOCK; restore blocking
                // mode so the per-connection read timeout governs reads.
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
                exhaust_flood = false;
                let finger = Arc::clone(&finger);
                thread::spawn(move || handle_connection(stream, &finger, peer));
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                // Poll tick: sleep, then loop back to the SIGTERM check.
                thread::sleep(POLL_TICK);
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                // Signal delivery raced the accept; loop back to the check.
            }
            Err(err) => {
                // Out of file descriptors (EMFILE/ENFILE) or similar. The
                // listen backlog still queues connecting clients, so drain
                // the queue by spending the reserved descriptor: a queued
                // client is accepted and refused immediately instead of
                // hanging until some fd happens to free.
                if !exhaust_flood {
                    eprintln!("thimbl: accept failed: {err}; refusing new clients while busy");
                    exhaust_flood = true;
                }
                if !refuse_one_backlogged(&listener, spare.take()) {
                    // Nothing in the backlog (or the spare was already
                    // spent): back off so the table can drain.
                    thread::sleep(ACCEPT_BACKOFF);
                }
                spare = listener.try_clone().ok();
            }
        }
    }
    ExitCode::SUCCESS
}

/// @planks("the server process has a file descriptor limit of {int}")
/// @planks("{int} clients connect and stay silent")
/// @planks("a new client queries the user's login name")
/// @planks("the new client receives a response or a refusal within {int} seconds")
/// Serves one client that is stuck in the listen backlog while the
/// descriptor table is full: spends the pre-reserved duplicate of the
/// listening socket's descriptor, accepts the queued client, sends the
/// busy refusal and closes. Returns `false` when no spare was held or no
/// client was queued, so the caller can back off instead.
fn refuse_one_backlogged(listener: &TcpListener, spare: Option<TcpListener>) -> bool {
    let Some(spare) = spare else {
        return false;
    };
    // Release the reserved descriptor first: accept needs a free slot to
    // create the connection's descriptor in.
    drop(spare);
    match listener.accept() {
        Ok((mut stream, peer)) => {
            let _ = stream.set_nonblocking(false);
            let _ = stream.set_write_timeout(Some(ACCEPT_BACKOFF));
            let _ = stream.write_all(BUSY_REPLY.as_bytes());
            let _ = stream.shutdown(std::net::Shutdown::Both);
            eprintln!("thimbl: {peer} refused: server busy");
            true
        }
        Err(_) => false,
    }
}

/// @planks("a client connects and sends an empty query")
/// @planks("the server closes the connection")
/// @planks("the connection is closed by the server within {int} seconds")
/// @planks("the server output contains a line naming the refused query")
/// @planks("the server output contains the client address")
/// Serves one connection: reads the query under the read timeout (a silent
/// client is reaped when it expires), answers, and logs one stderr line
/// naming the client address and the disposition.
fn handle_connection(mut stream: TcpStream, finger: &Finger, peer: SocketAddr) {
    // Reap silent clients: with the timeout set, the read loop fails with
    // `WouldBlock` after the window and the connection is dropped.
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    match read_query(&mut stream) {
        Err(_) => {
            // Read error or the read timeout: the connection is closed
            // without an answer; the timeout case is the reaping above.
            eprintln!("thimbl: {peer} connection lost before a query");
        }
        Ok(None) => {
            let _ = stream.write_all(format!("finger: query too long{CRLF}").as_bytes());
            eprintln!("thimbl: {peer} refused");
        }
        Ok(Some(line)) => {
            let reply = finger.answer(&line);
            let disposition = if reply.contains("forwarding service denied") {
                "refused"
            } else if reply.contains("no such user") {
                "no-match"
            } else {
                "answered"
            };
            let _ = stream.write_all(reply.as_bytes());
            eprintln!("thimbl: {peer} {disposition}");
        }
    }
}

/// @planks("a client connects and sends a query of 600 characters")
/// @planks("thimbl serves stdio with the query {string}")
/// @planks("thimbl serves stdio with the query of 600 characters")
/// Reads one query line from any byte source (a TCP connection or the
/// stdio serve's stdin), cut at the first LF with a single trailing CR
/// stripped, tolerating clients that close without a terminator.
/// Returns `None` when the line exceeds [`MAX_QUERY_BYTES`].
fn read_query(stream: &mut impl Read) -> io::Result<Option<String>> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        let end = chunk[..n]
            .iter()
            .position(|&byte| byte == b'\n')
            .unwrap_or(n);
        buf.extend_from_slice(&chunk[..end]);
        if buf.len() > MAX_QUERY_BYTES {
            return Ok(None);
        }
        if end < n {
            break;
        }
    }
    if buf.last() == Some(&b'\r') {
        buf.pop();
    }
    Ok(Some(String::from_utf8_lossy(&buf).into_owned()))
}

/// @planks("thimbl serves stdio with the query {string}")
/// @planks("thimbl serves stdio with the query of 600 characters")
/// @planks("the stdio answer contains the user's identity line")
/// @planks("the stdio answer contains the project content {string}")
/// @planks("the stdio answer contains the plan content {string}")
/// @planks("the stdio answer does not contain {string}")
/// @planks("the stdio answer contains {string}")
/// @planks("the stdio answer states that the user was not found")
/// @planks("it exits with the code {int}")
/// One connection of socket-activated serving: answers a single query
/// read from stdin to stdout, reusing the [`Finger`] answer logic, then
/// returns success so the supervisor's per-connection unit ends cleanly.
/// Nothing is printed at startup: the answer is the only stdout output.
fn serve_stdio() -> ExitCode {
    let finger = Finger::current();
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let reply = match read_query(&mut input) {
        Ok(Some(query)) => finger.answer(&query),
        // The same refusal a TCP connection gets for an overlong line.
        Ok(None) => format!("finger: query too long{CRLF}"),
        Err(err) => {
            eprintln!("thimbl: cannot read the stdio query: {err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = io::stdout().write_all(reply.as_bytes()) {
        eprintln!("thimbl: cannot write the stdio answer: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Everything the server serves: one identity plus the state directory
/// the card files are read from. Shared across connection threads.
struct Finger {
    identity: Identity,
    state_dir: Option<PathBuf>,
}

impl Finger {
    /// @planks("the finger server is running on an ephemeral port")
    /// @planks("the finger server is started on port 0")
    /// @planks("the response contains the user's identity line")
    fn current() -> Self {
        let home = env::var_os("HOME").map(PathBuf::from);
        let state_dir = home.as_deref().map(|home| home.join(".local/share/thimbl"));
        Self {
            identity: current_identity(),
            state_dir,
        }
    }

    /// @planks("a client connects and sends the query {string}")
    /// @planks("the response contains {string}")
    /// @planks("a client connects and sends the user's login name")
    /// @planks("a client connects and sends the user's real name")
    /// @planks("the response contains the user's login and real name")
    /// @planks("the response states that the user was not found")
    /// @planks("thimbl serves stdio with the query {string}")
    /// @planks("thimbl serves stdio with the query of 600 characters")
    /// @planks("the stdio answer contains {string}")
    /// @planks("the stdio answer states that the user was not found")
    /// Answers one query line: the local card for an empty query or a
    /// match on login/real name, the forwarding refusal, or the no-match
    /// notice. `/W` is accepted and ignored.
    fn answer(&self, query: &str) -> String {
        let query = query.trim();
        if query.contains('@') {
            return format!("Finger forwarding service denied{CRLF}");
        }
        let target = query.strip_prefix("/W").map_or(query, str::trim);
        if target.is_empty() || self.matches_user(target) {
            self.long_card()
        } else {
            format!("finger: no such user: {target}{CRLF}")
        }
    }

    /// @planks("a client connects and sends the name {string}")
    fn matches_user(&self, target: &str) -> bool {
        self.identity.login.eq_ignore_ascii_case(target)
            || self.identity.real_name.eq_ignore_ascii_case(target)
    }

    /// @planks("the response contains (?:a|an) "([^"]+)" line with (?:the )?(?:"([^"]+)"|(.+))")
    /// @planks("every line of the response ends with CRLF")
    /// @planks("the stdio answer contains the user's identity line")
    /// @planks("the stdio answer contains the project content {string}")
    /// @planks("the stdio answer contains the plan content {string}")
    /// The finger(1) long format: one identity line naming the resolved
    /// user, then the Project and Plan sections, read from the state
    /// directory at call time.
    fn long_card(&self) -> String {
        let mut card = format!("User: {}{CRLF}", self.identity());
        self.append_section(&mut card, "Project", ".project", "No Project.");
        self.append_section(&mut card, "Plan", ".plan", "No Plan.");
        card
    }

    /// @planks("the state directory user is changed to {string}")
    /// The resolved identity of the card: the state directory's `.user`
    /// file (read live, trimmed), then the passwd comment field, then
    /// the login; an empty value falls through to the next source.
    fn identity(&self) -> String {
        let sources = [
            self.user_file_identity(),
            Some(self.identity.real_name.clone()),
            Some(self.identity.login.clone()),
        ];
        sources
            .into_iter()
            .flatten()
            .find(|value| !value.is_empty())
            .unwrap_or_default()
    }

    /// @planks("the state directory has a user file with content {string}")
    /// The `.user` file content from the state directory, read per query
    /// and trimmed; `None` when the file is missing or blank.
    fn user_file_identity(&self) -> Option<String> {
        let content = fs::read_to_string(self.state_dir.as_ref()?.join(".user")).ok()?;
        let trimmed = content.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    }

    /// @planks("the response contains a {string} section with {string}")
    /// @planks("the response contains the project content {string}")
    /// @planks("the response contains the plan content {string}")
    /// @planks("the plan file content is changed to {string}")
    /// @planks("the response does not contain {string}")
    /// Appends one named section: the live file content when the file is
    /// present (an empty file serves an empty body, as finger(1) does),
    /// the fixed notice only for a missing card file.
    fn append_section(&self, card: &mut String, header: &str, file: &str, absent: &str) {
        card.push_str(header);
        card.push(':');
        card.push_str(CRLF);
        let content = self.state_dir.as_deref().and_then(|state_dir| {
            fs::read(state_dir.join(file))
                .ok()
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        });
        match content {
            Some(content) => append_body(card, &content),
            None => {
                card.push_str(absent);
                card.push_str(CRLF);
            }
        }
    }
}

/// @planks("every line of the response ends with CRLF")
/// Appends file content as CRLF-terminated lines: interior newlines are
/// normalized to CRLF and the file's own trailing newline is not doubled
/// into a blank line.
fn append_body(card: &mut String, content: &str) {
    for line in content.trim_end_matches(['\r', '\n']).split('\n') {
        card.push_str(line.trim_end_matches('\r'));
        card.push_str(CRLF);
    }
}

/// Identity fields from one `/etc/passwd` line.
struct Identity {
    login: String,
    real_name: String,
}

/// Identity of the user running the server, from their own `/etc/passwd`
/// line. The comment field is the card's identity fallback; the login
/// is the match key and the last identity fallback. Falls back to the
/// environment alone when `/etc/passwd` is unusable, so the failure is
/// loud rather than serving a wrong card.
fn current_identity() -> Identity {
    passwd_identity().unwrap_or_else(|| Identity {
        login: env::var("USER")
            .or_else(|_| env::var("LOGNAME"))
            .unwrap_or_default(),
        real_name: String::new(),
    })
}

/// @planks("the response contains the user's identity line")
/// Finds the invoking user's `/etc/passwd` line: by real uid when known,
/// otherwise by login name. Returns `None` when no line matches.
fn passwd_identity() -> Option<Identity> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    let uid = real_uid().map(|uid| uid.to_string());
    let login = env::var("USER")
        .or_else(|_| env::var("LOGNAME"))
        .unwrap_or_default();
    let mut by_login = None;
    for line in passwd.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 6 {
            continue;
        }
        if uid.as_deref() == Some(fields[2]) {
            return Some(identity_from(&fields));
        }
        if by_login.is_none() && fields[0] == login {
            by_login = Some(identity_from(&fields));
        }
    }
    by_login
}

fn identity_from(fields: &[&str]) -> Identity {
    Identity {
        login: fields[0].to_string(),
        real_name: fields
            .get(4)
            .map(|gecos| gecos.split_once(',').map_or(*gecos, |(name, _)| name))
            .unwrap_or_default()
            .to_string(),
    }
}

/// The real uid of this process via `/proc` (Linux). `None` when
/// unavailable.
fn real_uid() -> Option<u32> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let fields = line.strip_prefix("Uid:")?;
        fields.split_whitespace().next()?.parse().ok()
    })
}
