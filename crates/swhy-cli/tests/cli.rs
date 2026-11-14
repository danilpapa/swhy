//! E0, step 5 of docs/TZ.md: «`swhy --version` и `swhy mcp` с пустым списком
//! инструментов».

use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(20);

fn swhy() -> Command {
    Command::new(env!("CARGO_BIN_EXE_swhy"))
}

#[test]
fn version_prints_name_and_version() {
    let out = swhy().arg("--version").output().unwrap();

    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("swhy {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(String::from_utf8(out.stderr).unwrap(), "");
}

#[test]
fn help_lists_the_mcp_command() {
    let out = swhy().arg("--help").output().unwrap();

    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(help.contains("Usage: swhy"), "{help}");
    assert!(
        help.lines().any(|l| l.trim_start().starts_with("mcp ")),
        "{help}"
    );
}

#[test]
fn unknown_command_is_a_usage_error() {
    let out = swhy().arg("no-such-command").output().unwrap();

    assert_eq!(out.status.code(), Some(2));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "");
}

/// A running `swhy mcp` with line-delimited JSON-RPC on stdin/stdout.
struct Mcp {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    /// Every line the server wrote to stdout, in order.
    seen: Vec<String>,
}

impl Mcp {
    fn start() -> Self {
        let mut child = swhy()
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Mcp {
            child,
            stdin,
            lines,
            seen: Vec::new(),
        }
    }

    fn send(&mut self, message: Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    /// Next line of stdout, parsed. Panics when the server stays silent.
    fn receive(&mut self) -> Value {
        let line = self
            .lines
            .recv_timeout(TIMEOUT)
            .expect("swhy mcp did not answer in time");
        self.seen.push(line.clone());
        serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("stdout line is not JSON ({e}): {line:?}"))
    }

    fn request(&mut self, message: Value) -> Value {
        let id = message["id"].clone();
        self.send(message);
        let response = self.receive();
        assert_eq!(response["jsonrpc"], json!("2.0"), "{response}");
        assert_eq!(response["id"], id, "{response}");
        response
    }

    /// Closes stdin and waits for the process to exit. Returns the exit code,
    /// all stdout lines and stderr.
    fn finish(mut self) -> (Option<i32>, Vec<String>, String) {
        drop(self.stdin.take());
        let deadline = Instant::now() + TIMEOUT;
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                self.child.kill().unwrap();
                panic!("swhy mcp did not exit after stdin was closed");
            }
            thread::sleep(Duration::from_millis(10));
        };
        while let Ok(line) = self.lines.recv_timeout(Duration::from_millis(200)) {
            self.seen.push(line);
        }
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        (status.code(), std::mem::take(&mut self.seen), stderr)
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// stdout of an MCP server carries JSON-RPC and nothing else.
fn assert_only_json_rpc(lines: &[String]) {
    for line in lines {
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("stdout line is not JSON ({e}): {line:?}"));
        assert_eq!(value["jsonrpc"], json!("2.0"), "not JSON-RPC: {line}");
    }
}

#[test]
fn mcp_initialize_then_tools_list_is_empty() {
    let mut mcp = Mcp::start();

    let init = mcp.request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "swhy-test", "version": "0.0.1"}
        }
    }));
    assert_eq!(init["result"]["protocolVersion"], json!("2025-11-25"));
    assert_eq!(init["result"]["serverInfo"]["name"], json!("swhy"));
    assert_eq!(
        init["result"]["serverInfo"]["version"],
        json!(env!("CARGO_PKG_VERSION"))
    );
    assert!(
        init["result"]["capabilities"]["tools"].is_object(),
        "tools capability is missing: {init}"
    );

    mcp.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));

    let tools = mcp.request(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    assert_eq!(tools["result"]["tools"], json!([]));
    assert!(tools.get("error").is_none(), "{tools}");

    let (code, lines, stderr) = mcp.finish();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    // Two requests, two responses, no other output.
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_only_json_rpc(&lines);
}

#[test]
fn mcp_stateless_2026_07_28_tools_list_is_empty() {
    let meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "swhy-test", "version": "0.0.1"},
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    let mut mcp = Mcp::start();

    let discover = mcp.request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "server/discover",
        "params": {"_meta": meta}
    }));
    assert_eq!(
        discover["result"]["_meta"]["io.modelcontextprotocol/serverInfo"],
        json!({"name": "swhy", "version": env!("CARGO_PKG_VERSION")})
    );
    assert!(
        discover["result"]["supportedVersions"]
            .as_array()
            .is_some_and(|versions| versions.contains(&json!("2026-07-28"))),
        "{discover}"
    );
    assert!(
        discover["result"]["capabilities"]["tools"].is_object(),
        "{discover}"
    );

    let tools = mcp.request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {"_meta": meta}
    }));
    assert_eq!(tools["result"]["tools"], json!([]));

    let (code, lines, stderr) = mcp.finish();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_only_json_rpc(&lines);
}

#[test]
fn mcp_calling_a_tool_that_does_not_exist_is_an_error() {
    let mut mcp = Mcp::start();
    mcp.request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "swhy-test", "version": "0.0.1"}
        }
    }));
    mcp.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));

    let call = mcp.request(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {"name": "no_such_tool", "arguments": {}}
    }));
    assert!(call.get("result").is_none(), "{call}");
    assert!(call["error"]["code"].is_i64(), "{call}");

    let (code, lines, _stderr) = mcp.finish();
    assert_eq!(code, Some(0));
    assert_only_json_rpc(&lines);
}
