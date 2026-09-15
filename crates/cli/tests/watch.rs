//! Integration tests for the `sokonanoda watch` service stream
//! (`docs/design/compiler-service-events.md`): the `service.hello` handshake,
//! the canonical `file.didChange` opener, the `--doc` alias, per-file
//! independent versions under `--workspace`, and the client→service command
//! set (`ping`/`subscribe`/`unsubscribe`, error recovery). All waits are
//! bounded recv timeouts, never fixed sleeps.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;

use common::{WATCH_HANDSHAKE, WATCH_VOCABULARY};

mod common;

/// Upper bound for any single wait; polling/timeout only, no fixed sleeps.
const EVENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Short drain window used to prove "nothing more is coming" without sleeping
/// for a fixed long time.
const QUIET_WINDOW: Duration = Duration::from_millis(500);

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sokonanoda-watch-{tag}-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write_source(path: &Path, content: &str) {
    std::fs::write(path, content).expect("write source");
}

/// Atomically replace a file's contents so the poller never observes a
/// truncated/partial document (a rename never leaves a half-written file).
fn replace_source(path: &Path, content: &str) {
    let tmp = path.with_extension("sokonanoda.new");
    std::fs::write(&tmp, content).expect("write temp source");
    std::fs::rename(&tmp, path).expect("atomic replace");
}

/// A live `sokonanoda watch` child plus its parsed stdout event stream.
struct Watch {
    child: Child,
    stdin: Option<std::process::ChildStdin>,
    events: Receiver<Value>,
    _reader: JoinHandle<()>,
}

impl Watch {
    fn spawn(args: &[&str]) -> Self {
        Self::spawn_inner(args, Stdio::null())
    }

    /// Like [`Watch::spawn`], but keeps stdin open so the test can drive the
    /// client→service command set (docs/protocol.md, watch stream).
    fn spawn_with_stdin(args: &[&str]) -> Self {
        Self::spawn_inner(args, Stdio::piped())
    }

    fn spawn_inner(args: &[&str], stdin: Stdio) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sokonanoda"))
            .args(args)
            .stdin(stdin)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sokonanoda watch");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("child stdout");
        let (tx, events) = mpsc::channel();
        let _reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                match serde_json::from_str::<Value>(&line) {
                    Ok(value) => {
                        if tx.send(value).is_err() {
                            break;
                        }
                    }
                    Err(e) => panic!("watch stdout line is not JSON ({e}): {line:?}"),
                }
            }
        });
        Self {
            child,
            stdin,
            events,
            _reader,
        }
    }

    /// Send one JSON Lines command to the service.
    fn send(&mut self, line: &str) {
        use std::io::Write;
        let stdin = self.stdin.as_mut().expect("watch stdin is piped");
        writeln!(stdin, "{line}").expect("write watch command");
        stdin.flush().expect("flush watch command");
    }

    fn next(&self) -> Value {
        self.events
            .recv_timeout(EVENT_TIMEOUT)
            .unwrap_or_else(|_| panic!("timed out waiting for a watch event"))
    }

    /// Wait until a matching event arrives, discarding the rest.
    fn until(&self, mut pred: impl FnMut(&Value) -> bool) -> Value {
        let deadline = Instant::now() + EVENT_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.events.recv_timeout(remaining) {
                Ok(value) if pred(&value) => return value,
                Ok(_) => continue,
                Err(RecvTimeoutError::Timeout) => {
                    panic!("timed out waiting for a matching watch event")
                }
                Err(RecvTimeoutError::Disconnected) => panic!("watch stream ended early"),
            }
        }
    }

    /// Drain events until the stream goes quiet for `QUIET_WINDOW`.
    fn drain_quiet(&self) -> Vec<Value> {
        let mut drained = Vec::new();
        while let Ok(event) = self.events.recv_timeout(QUIET_WINDOW) {
            drained.push(event);
        }
        drained
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn event_type(event: &Value) -> &str {
    event.get("type").and_then(Value::as_str).unwrap_or("")
}

fn is_did_change_for(event: &Value, suffix: &str) -> bool {
    event_type(event) == "file.didChange"
        && event
            .get("file")
            .and_then(Value::as_str)
            .is_some_and(|file| file.ends_with(suffix))
}

#[test]
fn watch_stream_starts_with_the_service_hello_handshake() {
    let dir = temp_dir("hello");
    let file = dir.join("canvas.sokonanoda");
    write_source(&file, "def id : Prop -> Prop := fun (x : Prop) => x\n");

    let watch = Watch::spawn(&["watch", file.to_str().expect("utf-8 path")]);
    let hello = watch.next();

    assert_eq!(
        event_type(&hello),
        WATCH_HANDSHAKE,
        "first stream line must be the handshake: {hello}"
    );
    assert_eq!(hello["protocol"], 1, "protocol version: {hello}");
    assert_eq!(
        hello["engine"],
        env!("CARGO_PKG_VERSION"),
        "engine mirrors the crate version: {hello}"
    );
    let pid = hello["pid"].as_u64().expect("handshake pid is an integer");
    assert!(pid > 0, "handshake pid must be positive: {hello}");
    assert!(
        hello.get("version").is_none() && hello.get("file").is_none(),
        "handshake is service-level, not per-file: {hello}"
    );
}

#[test]
fn watch_emits_canonical_file_did_change_and_never_the_deprecated_alias() {
    let dir = temp_dir("didchange");
    let file = dir.join("canvas.sokonanoda");
    write_source(&file, "def id : Prop -> Prop := fun (x : Prop) => x\n");

    let watch = Watch::spawn(&["watch", file.to_str().expect("utf-8 path")]);
    let mut seen = vec![watch.next()];
    let opener = watch.until(|event| event_type(event) == "file.didChange");
    assert_eq!(opener["version"], 1, "first version is 1: {opener}");
    assert!(
        opener.get("recompiled_from").is_some(),
        "opener carries recompiled_from: {opener}"
    );
    assert_eq!(
        opener["file"],
        file.to_str().expect("utf-8 path"),
        "opener carries the stable file path: {opener}"
    );
    seen.push(opener);
    seen.extend(watch.drain_quiet());

    let types: Vec<&str> = seen.iter().map(event_type).collect();
    assert!(
        types.contains(&"file.didChange"),
        "canonical opener must be emitted: {types:?}"
    );
    assert!(
        !types.contains(&"file.changed"),
        "deprecated alias must never be emitted: {types:?}"
    );
}

#[test]
fn watch_doc_flag_is_an_alias_for_the_positional_path() {
    let dir = temp_dir("doc-alias");
    let file = dir.join("canvas.sokonanoda");
    write_source(&file, "def id : Prop -> Prop := fun (x : Prop) => x\n");

    let watch = Watch::spawn(&["watch", "--doc", file.to_str().expect("utf-8 path")]);
    assert_eq!(event_type(&watch.next()), WATCH_HANDSHAKE);
    let opener = watch.until(|event| event_type(event) == "file.didChange");
    assert_eq!(
        opener["file"],
        file.to_str().expect("utf-8 path"),
        "--doc points the session at the same file: {opener}"
    );
}

#[test]
fn watch_workspace_tracks_each_file_with_independent_versions() {
    let dir = temp_dir("workspace");
    let a = dir.join("a.sokonanoda");
    let b = dir.join("b.sokonanoda");
    write_source(&a, "def a : Prop -> Prop := fun (x : Prop) => x\n");
    write_source(&b, "def b : Prop -> Prop := fun (x : Prop) => x\n");

    let watch = Watch::spawn(&["watch", "--workspace", dir.to_str().expect("utf-8 path")]);
    assert_eq!(event_type(&watch.next()), WATCH_HANDSHAKE);

    let mut a_version = None;
    let mut b_version = None;
    let deadline = Instant::now() + EVENT_TIMEOUT;
    while (a_version.is_none() || b_version.is_none()) && Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = match watch.events.recv_timeout(remaining) {
            Ok(event) => event,
            Err(_) => break,
        };
        if is_did_change_for(&event, "a.sokonanoda") {
            a_version = event["version"].as_u64();
        }
        if is_did_change_for(&event, "b.sokonanoda") {
            b_version = event["version"].as_u64();
        }
    }
    assert_eq!(a_version, Some(1), "a's first version is 1");
    assert_eq!(b_version, Some(1), "b's first version is 1");

    replace_source(
        &a,
        "def a : Prop -> Prop := fun (x : Prop) => x\n#check a\n",
    );
    let bumped =
        watch.until(|event| is_did_change_for(event, "a.sokonanoda") && event["version"] == 2);
    assert_eq!(bumped["version"], 2, "editing a bumps only a: {bumped}");

    // b must stay at version 1: version counters are per file, no global order.
    for event in watch.drain_quiet() {
        if is_did_change_for(&event, "b.sokonanoda") {
            assert_eq!(
                event["version"], 1,
                "b's version must not be coupled to a's: {event}"
            );
        }
    }
}

#[test]
fn watch_event_names_stay_in_the_closed_vocabulary() {
    let dir = temp_dir("vocab");
    let file = dir.join("canvas.sokonanoda");
    write_source(&file, "def id : Prop -> Prop := fun (x : Prop) => x\n");

    let watch = Watch::spawn(&["watch", file.to_str().expect("utf-8 path")]);
    let mut seen = vec![watch.next()];
    seen.extend(watch.drain_quiet());

    for event in &seen {
        let t = event_type(event);
        assert!(
            t == WATCH_HANDSHAKE || WATCH_VOCABULARY.contains(&t),
            "watch emitted {t:?} outside the closed vocabulary: {event}"
        );
        if t != WATCH_HANDSHAKE {
            assert!(
                event.get("file").and_then(Value::as_str).is_some(),
                "per-file event must carry a stable `file` field: {event}"
            );
        }
    }
    assert!(
        seen.iter()
            .any(|event| event_type(event) == "file.didChange"),
        "a version opener must be emitted: {seen:?}"
    );
}

#[test]
fn protocol_document_lists_every_watch_event_name() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/protocol.md"
    ))
    .expect("read docs/protocol.md");
    for name in WATCH_VOCABULARY {
        assert!(
            doc.contains(name),
            "docs/protocol.md no longer documents watch event type {name:?}"
        );
    }
    assert!(
        doc.contains(WATCH_HANDSHAKE),
        "docs/protocol.md must document the {WATCH_HANDSHAKE:?} handshake"
    );
    for command in ["ping", "pong", "subscribe", "unsubscribe"] {
        assert!(
            doc.contains(command),
            "docs/protocol.md must document the watch command/response {command:?}"
        );
    }
}

#[test]
fn ping_round_trips_as_pong() {
    let dir = temp_dir("ping");
    let file = dir.join("canvas.sokonanoda");
    write_source(&file, "def id : Prop -> Prop := fun (x : Prop) => x\n");

    let mut watch = Watch::spawn_with_stdin(&["watch", file.to_str().expect("utf-8 path")]);
    assert_eq!(event_type(&watch.next()), WATCH_HANDSHAKE);

    watch.send(r#"{"type":"ping","id":42}"#);
    let pong = watch.until(|event| event_type(event) == "pong");
    assert_eq!(pong["id"], 42, "pong echoes the ping id: {pong}");
    assert_eq!(
        pong["protocol"], 1,
        "pong reports the control-plane protocol: {pong}"
    );
    assert_eq!(
        pong["engine"],
        env!("CARGO_PKG_VERSION"),
        "pong mirrors the engine version: {pong}"
    );
}

#[test]
fn subscribe_filters_workspace_events_to_the_named_file() {
    let dir = temp_dir("subscribe");
    let a = dir.join("a.sokonanoda");
    let b = dir.join("b.sokonanoda");
    write_source(&a, "def a : Prop -> Prop := fun (x : Prop) => x\n");
    write_source(&b, "def b : Prop -> Prop := fun (x : Prop) => x\n");

    let mut watch =
        Watch::spawn_with_stdin(&["watch", "--workspace", dir.to_str().expect("utf-8 path")]);
    assert_eq!(event_type(&watch.next()), WATCH_HANDSHAKE);
    watch.until(|event| is_did_change_for(event, "a.sokonanoda"));
    watch.until(|event| is_did_change_for(event, "b.sokonanoda"));
    watch.drain_quiet();

    let a_path = a.to_str().expect("utf-8 path");
    watch.send(&format!(r#"{{"type":"subscribe","file":"{a_path}"}}"#));
    // Commands are FIFO: the pong proves the subscription is already in
    // effect before the edits below are polled.
    watch.send(r#"{"type":"ping","id":1}"#);
    watch.until(|event| event_type(event) == "pong");

    replace_source(
        &a,
        "def a : Prop -> Prop := fun (x : Prop) => x\n#check a\n",
    );
    replace_source(
        &b,
        "def b : Prop -> Prop := fun (x : Prop) => x\n#check b\n",
    );

    let bumped =
        watch.until(|event| is_did_change_for(event, "a.sokonanoda") && event["version"] == 2);
    assert_eq!(bumped["file"], a_path, "a still emits: {bumped}");

    for event in watch.drain_quiet() {
        assert!(
            !(is_did_change_for(&event, "b.sokonanoda") && event["version"] == 2),
            "an unsubscribed workspace file must not emit: {event}"
        );
    }
}

#[test]
fn unsubscribe_stops_events_for_the_file() {
    let dir = temp_dir("unsubscribe");
    let file = dir.join("canvas.sokonanoda");
    write_source(&file, "def id : Prop -> Prop := fun (x : Prop) => x\n");

    let mut watch = Watch::spawn_with_stdin(&["watch", file.to_str().expect("utf-8 path")]);
    assert_eq!(event_type(&watch.next()), WATCH_HANDSHAKE);
    watch.until(|event| event_type(event) == "file.didChange");
    watch.drain_quiet();

    let path = file.to_str().expect("utf-8 path");
    watch.send(&format!(r#"{{"type":"subscribe","file":"{path}"}}"#));
    watch.send(r#"{"type":"ping","id":1}"#);
    watch.until(|event| event_type(event) == "pong");

    replace_source(
        &file,
        "def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n",
    );
    watch.until(|event| event_type(event) == "file.didChange" && event["version"] == 2);
    watch.drain_quiet();

    watch.send(&format!(r#"{{"type":"unsubscribe","file":"{path}"}}"#));
    watch.send(r#"{"type":"ping","id":2}"#);
    watch.until(|event| event_type(event) == "pong");

    replace_source(
        &file,
        "def id : Prop -> Prop := fun (x : Prop) => x\n#check id\n#check id\n",
    );
    // The stream stays alive while the file is filtered.
    watch.send(r#"{"type":"ping","id":3}"#);
    watch.until(|event| event_type(event) == "pong");

    for event in watch.drain_quiet() {
        assert!(
            !(event_type(&event) == "file.didChange" && event["version"] == 3),
            "an unsubscribed file must stop emitting: {event}"
        );
    }
}

#[test]
fn malformed_command_emits_an_error_and_does_not_kill_the_stream() {
    let dir = temp_dir("malformed");
    let file = dir.join("canvas.sokonanoda");
    write_source(&file, "def id : Prop -> Prop := fun (x : Prop) => x\n");

    let mut watch = Watch::spawn_with_stdin(&["watch", file.to_str().expect("utf-8 path")]);
    assert_eq!(event_type(&watch.next()), WATCH_HANDSHAKE);

    watch.send("this is not json");
    let error = watch.until(|event| event_type(event) == "error");
    assert!(
        error["message"].as_str().is_some_and(|m| !m.is_empty()),
        "malformed line yields a structured error: {error}"
    );

    watch.send(r#"{"type":"frobnicate"}"#);
    let error = watch.until(|event| event_type(event) == "error");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|m| m.contains("frobnicate")),
        "unknown command names the offending type: {error}"
    );

    watch.send(r#"{"type":"ping","id":9}"#);
    let pong = watch.until(|event| event_type(event) == "pong");
    assert_eq!(pong["id"], 9, "the stream survives bad commands: {pong}");
}
