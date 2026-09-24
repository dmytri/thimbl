//! Verification support for the thimbl cucumber harness.
//!
//! Owns three concerns the step definitions share:
//!
//! * identity — the invoking user's `/etc/passwd` line (login, real name,
//!   home, shell), which the scenarios query;
//! * fixtures — a temp `HOME` containing `.project`/`.plan` files the
//!   scenarios can create, edit and delete;
//! * lifecycle — one server process per scenario (started on an ephemeral
//!   port or on port 0), plus a raw TCP client for queries.

pub mod evidence;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Stops the server and removes the temp HOME. Registered as an After hook.
pub fn shutdown(world: &mut FingerWorld) {
    if let Some(server) = world.server.as_mut() {
        server.kill();
    }
    cleanup_home(&world.home);
}

/// Identity fields read once from the invoking user's `/etc/passwd` line.
#[derive(Clone, Debug)]
pub struct Identity {
    pub login: String,
    pub real_name: String,
    pub home: String,
    pub shell: String,
}

impl Identity {
    /// Reads the current user's passwd line. Falls back to environment
    /// variables if the line is malformed.
    pub fn current() -> Self {
        let login = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_default();
        let uid_line = passwd_line_for(&login);
        let (real_name, home, shell) = match &uid_line {
            Some(line) => {
                let fields: Vec<&str> = line.split(':').collect();
                let name = fields
                    .get(4)
                    .map(|g| g.split(',').next().unwrap_or("").to_string())
                    .unwrap_or_default();
                let home = fields.get(5).map(|s| s.to_string()).unwrap_or_default();
                let shell = fields.get(6).map(|s| s.to_string()).unwrap_or_default();
                (name, home, shell)
            }
            None => (
                String::new(),
                std::env::var("HOME").unwrap_or_default(),
                String::new(),
            ),
        };
        Self {
            login,
            real_name,
            home,
            shell,
        }
    }
}

fn passwd_line_for(login: &str) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    passwd
        .lines()
        .find(|line| line.split(':').next().map(|u| u == login).unwrap_or(false))
        .map(|line| line.to_string())
}

/// State of the fixture project/plan files for the running scenario.
#[derive(Clone, Debug, Default)]
pub struct FixtureFiles {
    pub project: Option<String>,
    pub plan: Option<String>,
}

/// Per-scenario world: identity, temp HOME fixtures, the server process and
/// the last response captured from a query.
#[derive(Debug, Default, cucumber::World)]
pub struct FingerWorld {
    pub identity: Option<Identity>,
    pub home: Option<PathBuf>,
    pub files: FixtureFiles,
    pub server: Option<ServerHandle>,
    pub startup_line: Option<String>,
    pub response: Option<Vec<u8>>,
    pub response_text: Option<String>,
}

/// A running thimbl server process and the port it bound.
#[derive(Debug)]
pub struct ServerHandle {
    child: Child,
    pub port: u16,
}

impl ServerHandle {
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.kill();
    }
}

impl FingerWorld {
    /// Ensures the identity is loaded (called by step functions).
    pub fn identity(&mut self) -> &Identity {
        self.identity.get_or_insert_with(Identity::current)
    }

    /// Creates the temp HOME fixture directory.
    pub fn temp_home(&mut self) -> PathBuf {
        if let Some(home) = &self.home {
            return home.clone();
        }
        let base =
            std::env::temp_dir().join(format!("thimbl-cuke-{}-{}", std::process::id(), nanos(),));
        std::fs::create_dir_all(&base).expect("create temp HOME");
        self.home = Some(base.clone());
        base
    }

    /// Writes `.project` with the given content (if `Some`) or removes it
    /// (if `None`), then records the fixture state.
    pub fn set_project(&mut self, content: Option<&str>) {
        let home = self.temp_home();
        let path = home.join(".project");
        match content {
            Some(text) => {
                std::fs::write(&path, text).expect("write .project");
                self.files.project = Some(text.to_string());
            }
            None => {
                let _ = std::fs::remove_file(&path);
                self.files.project = None;
            }
        }
    }

    /// Writes `.plan` with the given content (if `Some`) or removes it
    /// (if `None`), then records the fixture state.
    pub fn set_plan(&mut self, content: Option<&str>) {
        let home = self.temp_home();
        let path = home.join(".plan");
        match content {
            Some(text) => {
                std::fs::write(&path, text).expect("write .plan");
                self.files.plan = Some(text.to_string());
            }
            None => {
                let _ = std::fs::remove_file(&path);
                self.files.plan = None;
            }
        }
    }

    /// Starts the thimbl server with the temp HOME and waits until it
    /// prints its bound address. `port_arg` is `None` for the default
    /// (ephemeral port) startup or `Some("0")` for the port-0 variant.
    pub fn start_server(&mut self, port_arg: Option<&str>) -> u16 {
        if let Some(server) = &self.server {
            return server.port;
        }
        let home = self.temp_home();
        let mut cmd = Command::new(server_binary());
        cmd.env("HOME", &home);
        if let Some(port) = port_arg {
            cmd.arg("--port").arg(port);
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("spawn thimbl server");
        let mut stdout = BufReader::new(child.stdout.take().expect("server stdout piped"));
        let mut startup_line = String::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if Instant::now() > deadline {
                let _ = child.kill();
                panic!("server did not print its bound address in time");
            }
            let mut chunk = Vec::new();
            match stdout.read_until(b'\n', &mut chunk) {
                Ok(0) => std::thread::sleep(Duration::from_millis(20)),
                Ok(_) => {
                    startup_line.push_str(&String::from_utf8_lossy(&chunk));
                    let line = startup_line.trim_end();
                    if let Some(port) = parse_bound_port(line) {
                        self.startup_line = Some(line.to_string());
                        let handle = ServerHandle { child, port };
                        self.server = Some(handle);
                        return port;
                    }
                }
                Err(_) => {
                    let _ = child.kill();
                    panic!("error reading server stdout");
                }
            }
            if child.try_wait().map(|s| s.is_some()).unwrap_or(false) {
                let _ = child.kill();
                panic!("server exited before printing its bound address");
            }
        }
    }

    /// Reads one line of startup output (blocking with a timeout) — used by
    /// the port-0 scenario after the server has started.
    pub fn read_startup_line(&mut self) -> String {
        self.startup_line.clone().unwrap_or_default()
    }

    /// Connects to the server, sends `query` (appending CRLF as finger
    /// clients do), reads the full response until close, and stores it.
    pub fn query(&mut self, query: &str) {
        let port = self
            .server
            .as_ref()
            .map(|s| s.port)
            .expect("server is running");
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to server");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .expect("set write timeout");
        stream.write_all(query.as_bytes()).expect("send query");
        // The finger protocol query ends with CRLF.
        stream.write_all(b"\r\n").expect("send query terminator");
        stream.flush().expect("flush query");
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);
        self.response_text = Some(String::from_utf8_lossy(&response).into_owned());
        self.response = Some(response);
    }
}

/// Extracts the bound port from a server startup line like
/// `listening on 127.0.0.1:54321` (the last integer after a colon).
fn parse_bound_port(line: &str) -> Option<u16> {
    let after_colon = line.rsplit(':').next()?;
    after_colon.trim().parse::<u16>().ok()
}

fn nanos() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Locates the compiled thimbl binary (workspace target dir).
fn server_binary() -> PathBuf {
    // When run under `cargo test`, CARGO_BIN_EXE_ variables point at the
    // freshly built binaries — but only for libtest-harness targets.
    // For a harness=false integration test we resolve via the target dir.
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo");
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let path = PathBuf::from(manifest_dir)
        .join("target")
        .join(profile)
        .join("thimbl");
    if path.exists() {
        return path;
    }
    panic!(
        "thimbl binary not found at {} — build it first (cargo build)",
        path.display()
    );
}

/// Removes the temp HOME if present (called from an After hook).
pub fn cleanup_home(home: &Option<PathBuf>) {
    if let Some(home) = home {
        let _ = std::fs::remove_dir_all(home);
    }
}
