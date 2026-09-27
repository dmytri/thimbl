//! thimbl — a single-user RFC 1288 finger server.
//!
//! Serves exactly one card: the identity of the user running the server,
//! taken from that user's own `/etc/passwd` line, plus the `~/.project`
//! and `~/.plan` files. The dot-files are re-read from the `HOME` the
//! server was started with on every query, so edits show up immediately.
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

fn main() -> ExitCode {
    match port_from_args() {
        Ok(port) => serve(port),
        Err(message) => {
            eprintln!("thimbl: {message}");
            eprintln!("usage: thimbl [--port PORT]");
            ExitCode::from(2)
        }
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
    let mut args = env::args().skip(1);
    let mut port = 0;
    while let Some(arg) = args.next() {
        let raw = if arg == "--port" {
            args.next().ok_or("--port requires a value")?
        } else if let Some(raw) = arg.strip_prefix("--port=") {
            raw.to_string()
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
                // Poll tick: loop back to the SIGTERM check.
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
/// Reads one query line, cut at the first LF with a single trailing CR
/// stripped, tolerating clients that close without a terminator. Returns
/// `None` when the line exceeds [`MAX_QUERY_BYTES`].
fn read_query(stream: &mut TcpStream) -> io::Result<Option<String>> {
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

/// Everything the server serves: one identity plus the `HOME` the
/// dot-files are read from. Shared across connection threads.
struct Finger {
    identity: Identity,
    home: Option<PathBuf>,
}

impl Finger {
    /// @planks("the finger server is running on an ephemeral port")
    /// @planks("the finger server is started on port 0")
    fn current() -> Self {
        Self {
            identity: current_identity(),
            home: env::var_os("HOME").map(PathBuf::from),
        }
    }

    /// @planks("a client connects and sends the query {string}")
    /// @planks("the response contains {string}")
    /// @planks("a client connects and sends the user's login name")
    /// @planks("a client connects and sends the user's real name")
    /// @planks("the response contains the user's login and real name")
    /// @planks("the response states that the user was not found")
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

    /// @planks("the response contains the login, real name, directory and shell")
    /// @planks("the response contains (?:a|an) \"([^\"]+)\" line with the (.+)")
    /// @planks("every line of the response ends with CRLF")
    /// The finger(1) long format: identity lines followed by the Project
    /// and Plan sections, read from `HOME` at call time.
    fn long_card(&self) -> String {
        let mut card = String::new();
        for (label, value) in [
            ("Login name", self.identity.login.as_str()),
            ("In real life", self.identity.real_name.as_str()),
            ("Directory", self.identity.home.as_str()),
            ("Shell", self.identity.shell.as_str()),
        ] {
            card.push_str(label);
            card.push_str(": ");
            card.push_str(value);
            card.push_str(CRLF);
        }
        self.append_section(&mut card, "Project", ".project", "No Project.");
        self.append_section(&mut card, "Plan", ".plan", "No Plan.");
        card
    }

    /// @planks("the response contains a {string} section with {string}")
    /// @planks("the response contains the project content {string}")
    /// @planks("the response contains the plan content {string}")
    /// @planks("the plan file content is changed to {string}")
    /// @planks("the response does not contain {string}")
    /// Appends one named section: the live file content when present, the
    /// fixed notice otherwise.
    fn append_section(&self, card: &mut String, header: &str, file: &str, absent: &str) {
        card.push_str(header);
        card.push(':');
        card.push_str(CRLF);
        let content = self.home.as_deref().and_then(|home| {
            fs::read(home.join(file))
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
    home: String,
    shell: String,
}

/// Identity of the user running the server, from their own `/etc/passwd`
/// line. Falls back to the environment alone when `/etc/passwd` is
/// unusable, so the failure is loud rather than serving a wrong card.
fn current_identity() -> Identity {
    passwd_identity().unwrap_or_else(|| Identity {
        login: env::var("USER")
            .or_else(|_| env::var("LOGNAME"))
            .unwrap_or_default(),
        real_name: String::new(),
        home: String::new(),
        shell: String::new(),
    })
}

/// @planks("the response contains the login, real name, directory and shell")
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
        home: fields[5].to_string(),
        shell: fields.get(6).unwrap_or(&"").to_string(),
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
