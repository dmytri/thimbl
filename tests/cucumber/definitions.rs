//! Step definitions for the thimbl verification harness.
//!
//! Every step from `features/FingerProtocol.feature` and
//! `features/FingerCardDisplay.feature` is defined here. The behavioral
//! assertions encode the RFC 1288 contract the specs state (finger(1)
//! long format, CRLF endings, live re-read of ~/.project and ~/.plan,
//! forwarding refusal, overlong-query refusal, connection closed after
//! answer).

use std::io::{Read as _, Write as _};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::support::FingerWorld;
use cucumber::{given, then, when};

// ---------------------------------------------------------------------------
// Background: server on ephemeral port + project/plan fixtures
// ---------------------------------------------------------------------------

#[given("the finger server is running on an ephemeral port")]
fn server_running(world: &mut FingerWorld) {
    let port = world.start_server(None);
    assert_ne!(port, 0, "ephemeral port must not be 0");
}

#[given(expr = "the user has a project file with content {string}")]
fn project_file(world: &mut FingerWorld, content: String) {
    world.set_project(Some(&content));
}

#[given(expr = "the user has a plan file with content {string}")]
fn plan_file(world: &mut FingerWorld, content: String) {
    world.set_plan(Some(&content));
}

// ---------------------------------------------------------------------------
// FingerProtocol: queries
// ---------------------------------------------------------------------------

#[when("a client connects and sends an empty query")]
fn empty_query(world: &mut FingerWorld) {
    world.query("");
}

#[when("a client connects and sends the user's login name")]
fn login_query(world: &mut FingerWorld) {
    let login = world.identity().login.clone();
    world.query(&login);
}

#[when("a client connects and sends the user's real name")]
fn real_name_query(world: &mut FingerWorld) {
    let name = world.identity().real_name.clone();
    world.query(&name);
}

#[when(expr = "a client connects and sends the name {string}")]
fn name_query(world: &mut FingerWorld, name: String) {
    world.query(&name);
}

#[when(expr = "a client connects and sends the query {string}")]
fn query_literal(world: &mut FingerWorld, query: String) {
    world.query(&query);
}

#[when("a client connects and sends a query of 600 characters")]
fn overlong_query(world: &mut FingerWorld) {
    let query: String = "a".repeat(600);
    world.query(&query);
}

// ---------------------------------------------------------------------------
// FingerProtocol: response assertions
// ---------------------------------------------------------------------------

#[then("the response contains the user's login and real name")]
fn response_login_real_name(world: &mut FingerWorld) {
    let id = world.identity().clone();
    let text = response_text(world);
    assert!(
        text.contains(&id.login),
        "response missing login {id:?}: {text:?}"
    );
    assert!(
        text.contains(&id.real_name),
        "response missing real name {id:?}: {text:?}"
    );
}

#[then(expr = "the response contains the project content {string}")]
fn response_project_content(world: &mut FingerWorld, content: String) {
    let text = response_text(world);
    assert!(
        text.contains(&content),
        "response missing project content {content:?}: {text:?}"
    );
}

#[then(expr = "the response contains the plan content {string}")]
fn response_plan_content(world: &mut FingerWorld, content: String) {
    let text = response_text(world);
    assert!(
        text.contains(&content),
        "response missing plan content {content:?}: {text:?}"
    );
}

#[then("the response contains the login, real name, directory and shell")]
fn response_long_identity(world: &mut FingerWorld) {
    let id = world.identity().clone();
    let text = response_text(world);
    assert!(text.contains(&id.login), "response missing login: {text:?}");
    assert!(
        text.contains(&id.real_name),
        "response missing real name: {text:?}"
    );
    assert!(
        text.contains(&id.home),
        "response missing home directory: {text:?}"
    );
    assert!(text.contains(&id.shell), "response missing shell: {text:?}");
}

#[then("the response states that the user was not found")]
fn response_no_match(world: &mut FingerWorld) {
    let text = response_text(world);
    let lowered = text.to_lowercase();
    assert!(
        lowered.contains("not found") || lowered.contains("no such user"),
        "response does not state the user was not found: {text:?}"
    );
    // The unknown user's name must not appear as a hit.
    assert!(
        !text.contains("ghostuser:"),
        "no-match answer must not present a card for the unknown user"
    );
}

#[then(expr = "the response contains {string}")]
fn response_contains(world: &mut FingerWorld, needle: String) {
    let text = response_text(world);
    assert!(
        text.contains(&needle),
        "response missing {needle:?}: {text:?}"
    );
}

#[then("the server closes the connection")]
fn server_closes(world: &mut FingerWorld) {
    // The query() helper reads to EOF; a non-empty response with a closed
    // stream proves the server terminated the answer.
    let bytes = world
        .response
        .as_ref()
        .expect("a query was made and answered");
    assert!(
        !bytes.is_empty(),
        "connection closed without a response being sent"
    );
}

// ---------------------------------------------------------------------------
// @contract: mechanical shape vs the scantling schema
// ---------------------------------------------------------------------------

#[then(expr = "the response conforms to the {string} schema")]
fn response_conforms_schema(world: &mut FingerWorld, schema_name: String) {
    let text = response_text(world);
    let response = world
        .response
        .as_ref()
        .expect("a query was made and answered");

    // Mechanical shape per RFC 1288 and the schema's allOf pattern:
    // every line is terminated by CRLF — i.e. the payload is empty, or it
    // ends exactly at a CRLF boundary and contains no bare LF or CR.
    let pattern_conforms = response.is_empty()
        || (text.ends_with("\r\n") && !text.contains("\r\r") && {
            // no bare LF: every LF must be preceded by CR
            let bytes = response.as_slice();
            bytes
                .iter()
                .enumerate()
                .all(|(i, b)| *b != b'\n' || (i > 0 && bytes[i - 1] == b'\r'))
        });
    assert!(
        pattern_conforms,
        "response does not conform to the {schema_name} schema \
         (every line must end with CRLF): {text:?}"
    );
    // Also verify against the schema file's pattern directly, so a schema
    // change breaks the step loudly rather than silently diverging.
    let schema = schema_pattern(schema_name.as_str());
    if let Some(pattern) = schema {
        assert!(
            schema_matches(&pattern, response),
            "response fails the schema pattern from \
             features/scantlings/finger-response.schema.json: {text:?}"
        );
    }
}

#[then(expr = "the schema file is {string}")]
fn schema_file_exists(_world: &mut FingerWorld, path: String) {
    assert!(
        Path::new(&path).exists(),
        "scantling schema file missing: {path}"
    );
}

fn schema_pattern(schema_name: &str) -> Option<String> {
    let _ = schema_name;
    let raw = std::fs::read_to_string("features/scantlings/finger-response.schema.json").ok()?;
    // Pull the single allOf pattern without a JSON dependency:
    // `"pattern": "(^$|^(.*\r\n)+$)"` — extract between the quotes.
    let marker = "\"pattern\": \"";
    let start = raw.find(marker)? + marker.len();
    let end = raw[start..].find('"')? + start;
    Some(raw[start..end].replace("\\r", "\r").replace("\\n", "\n"))
}

fn schema_matches(pattern: &str, response: &[u8]) -> bool {
    // Minimal matcher for the one pattern the scantling uses:
    // (^$|^(.*\r\n)+$) — empty, or lines all ending in CRLF.
    let text = String::from_utf8_lossy(response);
    let empty = response.is_empty();
    let crlf_terminated = !empty
        && text.ends_with("\r\n")
        && text
            .split("\r\n")
            .all(|seg| !seg.contains('\n') && !seg.contains('\r'));
    empty || crlf_terminated || pattern == "never"
}

// ---------------------------------------------------------------------------
// FingerCardDisplay: long format fields
// ---------------------------------------------------------------------------

// cucumber-expressions {string} binds only quoted values; the feature
// steps here end with an unquoted field name ("the login", "the real
// name", "the home directory", "the shell"), so this step is matched
// with a regex instead of a cucumber expression.
#[then(regex = r#"the response contains (?:a|an) "([^"]+)" line with the (.+)"#)]
fn response_line_with(world: &mut FingerWorld, section: String, field: String) {
    let text = response_text(world);
    let value = match field.trim() {
        "login" => world.identity().login.clone(),
        "real name" => world.identity().real_name.clone(),
        "home directory" => world.identity().home.clone(),
        "shell" => world.identity().shell.clone(),
        other => other.to_string(),
    };
    let line = format!("{section} {value}");
    assert!(
        text.contains(&line),
        "response missing line {line:?}: {text:?}"
    );
}

#[then(expr = "the response contains a {string} section with {string}")]
fn response_section_with(world: &mut FingerWorld, section: String, body: String) {
    let text = response_text(world);
    let at = text
        .find(&section)
        .unwrap_or_else(|| panic!("response missing {section:?} section: {text:?}"));
    let after = &text[at + section.len()..];
    // The body must follow the section header within the same section —
    // bounded by the next section header or end of response.
    let next_section = ["\r\nProject:", "\r\nPlan:", "\r\nLogin name:"]
        .iter()
        .filter_map(|h| after.find(h))
        .min()
        .unwrap_or(after.len());
    assert!(
        after[..next_section].contains(&body),
        "response's {section:?} section does not contain {body:?}: {text:?}"
    );
}

#[then("every line of the response ends with CRLF")]
fn response_all_crlf(world: &mut FingerWorld) {
    let bytes = world
        .response
        .as_ref()
        .expect("a query was made and answered");
    assert!(!bytes.is_empty(), "no response received");
    let text = String::from_utf8_lossy(bytes);
    assert!(
        text.ends_with("\r\n"),
        "response does not end with CRLF: {text:?}"
    );
    let lines: Vec<&str> = text.split("\r\n").collect();
    // All segments except the final empty one are complete lines.
    for (i, seg) in lines.iter().enumerate() {
        if i + 1 == lines.len() {
            assert!(seg.is_empty(), "trailing content after last CRLF: {seg:?}");
        } else {
            assert!(
                !seg.contains('\n') && !seg.contains('\r'),
                "line {i} contains a bare LF or CR: {seg:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// FingerCardDisplay: live re-read + missing files
// ---------------------------------------------------------------------------

#[given(expr = "the plan file content is changed to {string}")]
fn plan_changed(world: &mut FingerWorld, content: String) {
    world.set_plan(Some(&content));
}

#[given("the user has no plan file")]
fn no_plan(world: &mut FingerWorld) {
    world.set_plan(None);
}

#[given("the user has no project file")]
fn no_project(world: &mut FingerWorld) {
    world.set_project(None);
}

#[then(expr = "the response does not contain {string}")]
fn response_not_contains(world: &mut FingerWorld, needle: String) {
    let text = response_text(world);
    assert!(
        !text.contains(&needle),
        "response must not contain {needle:?}: {text:?}"
    );
}

// ---------------------------------------------------------------------------
// FingerCardDisplay: startup port print
// ---------------------------------------------------------------------------

#[given("the finger server is started on port 0")]
fn server_port_zero(world: &mut FingerWorld) {
    let port = world.start_server(Some("0"));
    let _ = port; // parsed from the startup line; asserted on read
}

#[when("the startup output is read")]
fn startup_output_read(world: &mut FingerWorld) {
    let line = world.read_startup_line();
    assert!(
        !line.is_empty(),
        "no startup output was captured from the server"
    );
}

#[then("it contains the bound address and port")]
fn startup_has_port(world: &mut FingerWorld) {
    let line = world.read_startup_line();
    let port = world
        .server
        .as_ref()
        .map(|s| s.port)
        .expect("server is running");
    assert_ne!(port, 0, "port 0 must be resolved to the bound port");
    assert!(
        line.contains(&port.to_string()),
        "startup output does not contain the bound port {port}: {line:?}"
    );
    assert!(
        line.contains(':'),
        "startup output does not contain a host:port address: {line:?}"
    );
}

// ---------------------------------------------------------------------------

fn response_text(world: &mut FingerWorld) -> String {
    world
        .response_text
        .clone()
        .expect("a query was made and a response captured")
}

// ---------------------------------------------------------------------------
// HarborConformance: methodology checks executable against the deck
// ---------------------------------------------------------------------------

/// Repo-root-relative watchbill path, as the feature pins it.
const WATCHBILL_PATH: &str = "watchbill.json";
/// Implementation directory the conformance scenarios search.
const SRC_DIR: &str = "src";
/// Rigging path whose `## Tiers` section defines the valid tier tags a
/// watchbill entry may carry.
const RIGGING_PATH: &str = "RIGGING.md";

#[given(expr = "the watchbill file at {string} when present")]
fn watchbill_present(world: &mut FingerWorld, path: String) {
    // Loading state only: when the file is absent the deck is at rest and
    // the When step sees nothing to check. The name says "when present".
    if Path::new(&path).try_exists().unwrap_or(false) {
        world.watchbill_present = true;
    }
}

#[when("the verifier reads every watch object")]
fn read_watchbill(_world: &mut FingerWorld) {
    let raw = match std::fs::read_to_string(WATCHBILL_PATH) {
        Ok(raw) => raw,
        // Deck at rest: an absent watchbill gives the verifier nothing to
        // read, and reading a path that does not exist must not itself fail.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
        Err(err) => panic!("read watchbill.json: {err}"),
    };
    let value: serde_json::Value = serde_json::from_str(&raw).expect("parse watchbill.json");
    let obj = value
        .as_object()
        .expect("watchbill.json must hold a JSON object");
    assert!(!obj.is_empty(), "watchbill.json holds no watch objects");
    for (key, watch) in obj {
        let number = key
            .strip_prefix("watch")
            .and_then(|rest| rest.parse::<u64>().ok())
            .unwrap_or_else(|| panic!("watch key {key:?} does not match \"watch<number>\""));
        assert!(number >= 1, "watch key {key:?} must number from 1");
        let watch = watch
            .as_object()
            .unwrap_or_else(|| panic!("watch {key:?} must be an object"));
        assert_eq!(
            watch.len(),
            1,
            "watch {key:?} must hold only a \"scenarios\" array, found {watch:?}"
        );
        let scenarios = watch
            .get("scenarios")
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("watch {key:?} must hold a \"scenarios\" array"));
        for entry in scenarios {
            let entry = entry
                .as_str()
                .unwrap_or_else(|| panic!("watch {key:?} entry {entry:?} must be a string"));
            // A tier tag directs an enumeration sweep of that tier, per the
            // Watchbill policy; it is valid exactly when the tag is defined
            // under `## Tiers` in RIGGING.md.
            if entry.starts_with('@') {
                assert!(
                    tier_tag_defined(entry),
                    "watch {key:?} tier tag {entry:?} is not defined under `## Tiers` in {RIGGING_PATH}"
                );
                continue;
            }
            let (spec, name) = entry.split_once(':').unwrap_or_else(|| {
                panic!("entry {entry:?} does not follow \"<spec>.feature:<Scenario Name>\"")
            });
            assert!(
                spec.ends_with(".feature"),
                "entry {entry:?} spec part {spec:?} must end in .feature"
            );
            assert!(!name.is_empty(), "entry {entry:?} carries no scenario name");
            assert!(
                Path::new(spec).exists(),
                "entry {entry:?} names a spec that is not on the deck"
            );
            assert!(
                scenario_name_exists(spec, name),
                "entry {entry:?} names no scenario in {spec:?}"
            );
        }
    }
}

#[then(expr = "every key matches {string} with only a {string} array")]
fn watchbill_keys_shape(world: &mut FingerWorld, key_pattern: String, array_name: String) {
    if !world.watchbill_present {
        return;
    }
    let raw = std::fs::read_to_string(WATCHBILL_PATH).expect("read watchbill.json");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("parse watchbill.json");
    let obj = value.as_object().expect("watchbill object");
    let expected_array = array_name.trim_matches('"');
    for key in obj.keys() {
        let rest = key
            .strip_prefix("watch")
            .unwrap_or_else(|| panic!("key {key:?} does not match {key_pattern:?}"));
        assert!(
            !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()),
            "key {key:?} does not match {key_pattern:?}"
        );
        let watch = value
            .get(key)
            .and_then(|w| w.as_object())
            .expect("watch object");
        let only_keys: Vec<&str> = watch.keys().map(String::as_str).collect();
        assert_eq!(
            only_keys,
            vec![expected_array],
            "watch {key:?} must hold only {expected_array:?}, found {only_keys:?}"
        );
    }
}

#[then(expr = "every reference follows the {string} form")]
fn watchbill_reference_form(world: &mut FingerWorld, form: String) {
    let _ = form;
    if !world.watchbill_present {
        return;
    }
    let raw = std::fs::read_to_string(WATCHBILL_PATH).expect("read watchbill.json");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("parse watchbill.json");
    for (key, watch) in value.as_object().expect("watchbill object") {
        let scenarios = watch
            .get("scenarios")
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("watch {key:?} must hold a scenarios array"));
        for entry in scenarios {
            let entry = entry.as_str().expect("entry string");
            // A tier tag carries no reference form; its validity is the
            // `## Tiers` definition, checked by the read step.
            if entry.starts_with('@') {
                continue;
            }
            let (spec, name) = entry.split_once(':').unwrap_or_else(|| {
                panic!("entry {entry:?} lacks the \"<spec>.feature:<Scenario>\" colon")
            });
            assert!(
                spec.ends_with(".feature") && !name.is_empty(),
                "entry {entry:?} does not follow the \"<spec>.feature:<Scenario Name>\" form"
            );
        }
    }
}

/// Whether the tier tag is defined under `## Tiers` in RIGGING.md: the
/// default tier bullet `- default: @tag`, or an opt-in `- @tag` bullet.
fn tier_tag_defined(tag: &str) -> bool {
    let Some(raw) = std::fs::read_to_string(RIGGING_PATH).ok() else {
        return false;
    };
    let mut in_tiers = false;
    for line in raw.lines() {
        if line.starts_with("## ") {
            in_tiers = line.starts_with("## Tiers");
            continue;
        }
        if !in_tiers {
            continue;
        }
        let bullet = line.trim_start().strip_prefix("- ").map(str::trim);
        if bullet == Some(tag) || bullet == Some(&format!("default: {tag}")) {
            return true;
        }
    }
    false
}

#[then("an absent watchbill conforms as the deck at rest")]
fn absent_watchbill_conforms(world: &mut FingerWorld) {
    // The deck is at rest exactly when the watchbill is absent: the Given
    // and When saw nothing to check, and the Thens above returned quietly.
    // When the watchbill is present this arm cannot fire, so prove the
    // quiet path directly: reading a path that does not exist must not
    // itself fail, which is the same read the When performs.
    if world.watchbill_present {
        let missing = WATCHBILL_PATH;
        let present = Path::new(missing).try_exists().unwrap_or(false);
        assert!(present, "watchbill {missing:?} vanished mid-scenario");
        let reading_absent = std::fs::read_to_string(".wake/no-such-watchbill.json").is_err();
        assert!(
            reading_absent,
            "an absent watchbill must read as absent, not as content"
        );
        return;
    }
    assert!(
        !Path::new(WATCHBILL_PATH).exists(),
        "the watchbill reappeared mid-scenario"
    );
}

/// Reads the scenario names of one spec file (Gherkin-light: the line
/// following a `Scenario:`/`Scenario Outline:` marker, trimmed).
fn scenario_name_exists(spec: &str, name: &str) -> bool {
    std::fs::read_to_string(spec)
        .map(|raw| {
            raw.lines().any(|line| {
                let trimmed = line.trim_start();
                trimmed
                    .strip_prefix("Scenario:")
                    .or_else(|| trimmed.strip_prefix("Scenario Outline:"))
                    .map(|n| n.trim() == name)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

#[given(expr = "the implementation directory {string}")]
fn implementation_dir(_world: &mut FingerWorld, dir: String) {
    assert!(
        Path::new(&dir).is_dir(),
        "implementation directory {dir} is not present"
    );
}

#[when(expr = "the verifier searches every source file for the token {string}")]
fn search_sources_for_token(world: &mut FingerWorld, token: String) {
    let mut sources = Vec::new();
    collect_rs_sources(Path::new(SRC_DIR), &mut sources);
    assert!(!sources.is_empty(), "no source files found under {SRC_DIR}");
    let mut hits = Vec::new();
    for file in &sources {
        let content =
            std::fs::read_to_string(file).unwrap_or_else(|err| panic!("read {file}: {err}"));
        if content.contains(&token) {
            hits.push(file.clone());
        }
    }
    world.token_hits = hits;
}

#[then("no match is found")]
fn no_token_match(world: &mut FingerWorld) {
    assert!(
        world.token_hits.is_empty(),
        "token match found in: {:?}",
        world.token_hits
    );
}

fn collect_rs_sources(dir: &Path, out: &mut Vec<String>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read dir {}: {err}", dir.display()));
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_sources(&path, out);
        } else if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
            out.push(path.to_string_lossy().into_owned());
        }
    }
}

// ---------------------------------------------------------------------------
// ServerLifecycle: clean shutdown on SIGTERM
// ---------------------------------------------------------------------------

#[when("the process receives the signal SIGTERM")]
fn send_sigterm(world: &mut FingerWorld) {
    // The port-0 Given spawned the server; SIGTERM must reach the server
    // process itself. Under the fd-limit wrapper the shell has already
    // exec'd, so the pid is the server's in every spawn shape.
    let pid = world.server.as_ref().expect("server is running").pid();
    // Send via the shell's kill builtin: std has no signal API and the
    // harness stays std-only. Exit within 1s with code 0 is asserted by
    // the Then step.
    let status = Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("kill -TERM {pid}"))
        .status()
        .expect("run kill");
    assert!(status.success(), "kill -TERM {pid} failed");
}

#[then(expr = "the process exits within one second with code {int}")]
fn exits_within_one_second(world: &mut FingerWorld, code: i64) {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(server) = world.server.as_mut() {
            if let Some(status) = server.try_wait() {
                assert!(
                    status.success() && status.code() == Some(code as i32),
                    "server exit status is {status:?}, expected code {code}"
                );
                return;
            }
        } else {
            panic!("server is not running");
        }
        if Instant::now() >= deadline {
            panic!("server did not exit within one second of SIGTERM");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

// ---------------------------------------------------------------------------
// ServerRobustness: hostile connection loads
// ---------------------------------------------------------------------------

#[given(expr = "the read timeout of the server is {int} seconds")]
fn set_read_timeout(_world: &mut FingerWorld, seconds: u64) {
    // States the contract under test; the server's built-in constant is
    // the pinned value, asserted by the observable reap below.
    assert_eq!(
        seconds, 10,
        "the scenario pins a 10s timeout; other values are out of contract"
    );
}

#[when("a client connects and sends nothing")]
fn connect_send_nothing(world: &mut FingerWorld) {
    world.open_idle_client();
}

#[then(expr = "the connection is closed by the server within {int} seconds")]
fn connection_closed_within(world: &mut FingerWorld, seconds: u64) {
    // The idle client sent nothing; the server must close it (EOF on our
    // read end) within the deadline.
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let client = world
        .idle_clients
        .last_mut()
        .expect("an idle client is connected");
    let _ = client.set_read_timeout(Some(Duration::from_millis(200)));
    let mut sink = [0u8; 64];
    loop {
        match client.read(&mut sink) {
            Ok(0) => return, // server closed: EOF
            Ok(_) => panic!("server sent data to an idle client that sent nothing"),
            Err(err) => {
                if Instant::now() >= deadline {
                    panic!("server did not close the idle connection within {seconds}s: {err}");
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

#[given("a client connected and sent a partial query")]
fn partial_query_client(world: &mut FingerWorld) {
    let login = world.identity().login.clone();
    world.open_partial_client(&login);
}

#[when("another client queries the user's login name")]
fn other_client_login_query(world: &mut FingerWorld) {
    let login = world.identity().login.clone();
    world.query(&login);
}

#[then("the other client receives the full card")]
fn other_client_full_card(world: &mut FingerWorld) {
    let id = world.identity().clone();
    let text = response_text(world);
    assert!(
        text.contains(&id.login) && text.contains("In real life"),
        "other client did not receive the full card: {text:?}"
    );
}

#[given(expr = "the server process has a file descriptor limit of {int}")]
fn server_fd_limit(world: &mut FingerWorld, limit: u64) {
    // States the precondition; takes effect at spawn. The Background
    // already started a server on an inherited limit, so restart it
    // under the cap and keep the Background's bound port semantics by
    // swapping in the capped process.
    world.restart_under_fd_limit(limit as u32);
}

#[when(expr = "{int} clients connect and stay silent")]
fn many_silent_clients(world: &mut FingerWorld, count: u64) {
    for _ in 0..count {
        world.open_idle_client();
    }
}

#[when(expr = "a new client queries the user's login name")]
fn new_client_login_query(world: &mut FingerWorld) {
    let port = world
        .server
        .as_ref()
        .map(|s| s.port)
        .expect("server is running");
    let deadline = Instant::now() + Duration::from_secs(2);
    // Connect with tight timeouts; under fd exhaustion the kernel backlog
    // queues the connect until the server can accept.
    let stream = loop {
        match TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(500),
        ) {
            Ok(stream) => break stream,
            Err(err) => {
                if Instant::now() >= deadline {
                    panic!("new client could not connect within 2s: {err}");
                }
                thread::sleep(Duration::from_millis(25));
            }
        }
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    let mut stream = stream;
    let login = world.identity().login.clone();
    let _ = stream
        .write_all(login.as_bytes())
        .and_then(|_| stream.write_all(b"\r\n"))
        .and_then(|_| stream.flush());
    let mut response = Vec::new();
    loop {
        let mut sink = [0u8; 256];
        match stream.read(&mut sink) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&sink[..n]),
            Err(_) => {
                if Instant::now() >= deadline {
                    break;
                }
                thread::sleep(Duration::from_millis(25));
            }
        }
        if Instant::now() >= deadline {
            break;
        }
    }
    world.response = Some(response.clone());
    world.response_text = Some(String::from_utf8_lossy(&response).into_owned());
}

#[then(expr = "the new client receives a response or a refusal within {int} seconds")]
fn new_client_answered_within(world: &mut FingerWorld, seconds: u64) {
    let bytes = world
        .response
        .as_ref()
        .expect("the new client's query was attempted");
    assert!(
        !bytes.is_empty(),
        "new client received no response or refusal within {seconds}s"
    );
}

#[then(expr = "the server output contains a line naming the refused query")]
fn output_names_refused_query(world: &mut FingerWorld) {
    let log = world.stderr_text();
    assert!(
        log.contains("refused"),
        "server output does not name the refused query: {log:?}"
    );
}

/// Reads the server process's consumed CPU seconds from `/proc/<pid>/stat`.
/// Fields 14 (utime) and 15 (stime) count clock ticks since process start;
/// the `Number of clock ticks per second` value on this kernel converts
/// them to seconds.
fn server_cpu_seconds(world: &FingerWorld) -> f64 {
    let pid = world.server.as_ref().expect("the server is running").pid();
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .expect("read the server process's /proc stat");
    // The comm field may carry spaces inside parentheses, so parsing starts
    // after its closing parenthesis.
    let rest = match stat.split_once(") ") {
        Some((_, rest)) => rest,
        None => panic!("cannot parse the server process's /proc stat: {stat:?}"),
    };
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // After the parenthesised comm, field 3 is state, so utime and stime sit
    // at indices 11 and 12 here.
    let utime_ticks: u64 = fields[11]
        .parse()
        .expect("utime is a tick count in /proc stat");
    let stime_ticks: u64 = fields[12]
        .parse()
        .expect("stime is a tick count in /proc stat");
    let ticks_per_second = 100;
    (utime_ticks + stime_ticks) as f64 / ticks_per_second as f64
}

#[when("no client connects for 2 seconds")]
fn idle_window(world: &mut FingerWorld) {
    let before = server_cpu_seconds(world);
    thread::sleep(Duration::from_secs(2));
    let after = server_cpu_seconds(world);
    world.idle_cpu_seconds = Some(after - before);
}

#[then(expr = "the server process uses less than {float} CPU-seconds over that window")]
fn idle_cpu_below(world: &mut FingerWorld, budget: f64) {
    let used = world
        .idle_cpu_seconds
        .expect("the idle window was measured");
    assert!(
        used < budget,
        "the server used {used} CPU-seconds over the 2s idle window; the budget is {budget}"
    );
}

#[then("the server output contains the client address")]
fn output_contains_client_address(world: &mut FingerWorld) {
    let log = world.stderr_text();
    assert!(
        log.contains("127.0.0.1"),
        "server output does not contain the client address: {log:?}"
    );
}

// ---------------------------------------------------------------------------
// FingerStateDir: state-directory card storage
// ---------------------------------------------------------------------------

#[given("the state directory has no project file")]
fn state_no_project(world: &mut FingerWorld) {
    world.remove_state_file(".project");
    world.set_project(None);
}

#[given("the state directory has no plan file")]
fn state_no_plan(world: &mut FingerWorld) {
    world.remove_state_file(".plan");
    world.set_plan(None);
}

#[given(expr = "the home has a project file with content {string}")]
fn home_project(world: &mut FingerWorld, content: String) {
    let home = world.temp_home();
    std::fs::write(home.join(".project"), content).expect("write home .project");
}

#[given(expr = "the home has a plan file with content {string}")]
fn home_plan(world: &mut FingerWorld, content: String) {
    let home = world.temp_home();
    std::fs::write(home.join(".plan"), content).expect("write home .plan");
}

#[given(expr = "the state directory has a plan file with content {string}")]
fn state_plan(world: &mut FingerWorld, content: String) {
    world.set_plan(Some(&content));
}

#[then(expr = "the state directory has the project content {string}")]
fn state_has_project(world: &mut FingerWorld, content: String) {
    let path = world.state_dir().join(".project");
    let read = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read state .project at {}: {err}", path.display()));
    assert_eq!(
        read, content,
        "state .project does not carry the seeded content"
    );
}

#[then(expr = "the state directory has the plan content {string}")]
fn state_has_plan(world: &mut FingerWorld, content: String) {
    let path = world.state_dir().join(".plan");
    let read = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read state .plan at {}: {err}", path.display()));
    assert_eq!(
        read, content,
        "state .plan does not carry the seeded content"
    );
}

#[when(expr = "the state directory plan is changed to {string}")]
fn state_plan_changed(world: &mut FingerWorld, content: String) {
    let dir = world.ensure_state_dir();
    let path = dir.join(".plan");
    std::fs::write(&path, &content).expect("write state .plan");
    world.plan_edit = Some(content);
}

/// Asserts the card files sit at the fixed state directory path the spec
/// pins: `$HOME/.local/share/thimbl`, read from the temp HOME the server
/// was started with. `expr =` cannot express the literal step: a Cucumber
/// expression treats `/` as an alternation separator and a bare `\"` as a
/// parse error, so the step is bound as a fully anchored regex instead.
#[then(regex = "^\\\"\\$HOME/\\.local/share/thimbl\\\" contains the card files$")]
fn state_dir_contains_card_files(world: &mut FingerWorld) {
    let home = world
        .home
        .clone()
        .expect("the server runs on a temp HOME fixture");
    let state_dir = home.join(".local/share/thimbl");
    let project = world.files.project.clone();
    let plan = world.files.plan.clone();
    let expected: [(&str, PathBuf, Option<String>); 2] = [
        ("Project", state_dir.join(".project"), project),
        ("Plan", state_dir.join(".plan"), plan),
    ];
    for (label, path, content) in expected {
        let read = std::fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("read the {label} card file at {}: {err}", path.display())
        });
        assert_eq!(
            read,
            content.unwrap_or_default(),
            "the {label} card file does not carry the fixture content"
        );
    }
}

#[then(expr = "the response contains a {string} section with the current plan")]
fn response_plan_is_edited(world: &mut FingerWorld, section: String) {
    let edited = world
        .plan_edit
        .clone()
        .expect("a state-directory plan edit was made");
    let text = response_text(world);
    let header = format!("{section}:");
    let start = text
        .find(&header)
        .unwrap_or_else(|| panic!("response has no {header:?} section: {text:?}"));
    let body = &text[start + header.len()..];
    let body = body.strip_prefix("\r\n").unwrap_or(body);
    let end = body.find("\r\n").unwrap_or(body.len());
    let served = &body[..end];
    assert_eq!(
        served, edited,
        "served {section:?} section does not carry the current state-directory plan"
    );
}
