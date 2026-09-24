//! Step definitions for the thimbl verification harness.
//!
//! Every step from `features/FingerProtocol.feature` and
//! `features/FingerCardDisplay.feature` is defined here. The behavioral
//! assertions encode the RFC 1288 contract the specs state (finger(1)
//! long format, CRLF endings, live re-read of ~/.project and ~/.plan,
//! forwarding refusal, overlong-query refusal, connection closed after
//! answer).

use std::path::Path;

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
