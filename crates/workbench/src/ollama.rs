//! The session loop. It talks only to a local Ollama on 127.0.0.1.
//!
//! Cloud tags are refused. The model it used is named in the reply, with Ollama's
//! digest for it, so a session can be traced to the weights that ran it.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::Value;

use crate::session::{MAX_CALLS, MAX_CALLS_PER_ROUND, MAX_ROUNDS, Phase, Workbench};

/// What a workbench session came back with.
pub enum BenchReply {
    Done {
        model: String,
        /// Ollama's digest for the model, when its tag list gave one.
        digest: Option<String>,
        bench: Box<Workbench>,
        stopped: String,
    },
    Absent(String),
}

const OLLAMA: u16 = 11434;

/// Run a workbench session on a background thread against the local Ollama.
pub fn start_bench(bench: Workbench) -> std::sync::mpsc::Receiver<BenchReply> {
    start_bench_on(OLLAMA, bench)
}

pub fn start_bench_on(port: u16, bench: Workbench) -> std::sync::mpsc::Receiver<BenchReply> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("workbench".to_string())
        .spawn(move || {
            let _ = tx.send(run_bench(port, bench));
        });
    rx
}

/// The chat loop: offer the tools, run what the model calls, hand back the program's answers.
fn run_bench(port: u16, mut bench: Workbench) -> BenchReply {
    let tags = match http(port, "GET", "/api/tags", None, Duration::from_secs(4)) {
        Ok(tags) => tags,
        Err(text) => return BenchReply::Absent(text),
    };
    let Some(model) = choose_tool_model(port, &local_model_names(&tags)) else {
        return BenchReply::Absent(
            "No local model that can call tools is available. Cloud tags are not used.".to_string(),
        );
    };
    let digest = model_digest(&tags, &model);
    let mut messages = vec![
        serde_json::json!({"role": "system", "content": bench.system_prompt()}),
        serde_json::json!({"role": "user", "content": bench.opening()}),
    ];
    let mut stopped = format!("It used all {} rounds.", MAX_ROUNDS);
    // A model that answers in prose is reminded once to use the tools.
    let mut reminded = false;
    for round in 0..MAX_ROUNDS {
        let phase = Phase::of(round);
        if round > 0 && Phase::of(round - 1) != phase {
            messages
                .push(serde_json::json!({"role": "user", "content": bench.phase_prompt(phase)}));
        }
        let payload = serde_json::json!({
            "model": model,
            "messages": messages,
            "tools": bench.tool_specs_for(phase),
            "stream": false,
            "think": false,
            "options": { "temperature": 0.2, "num_predict": 600 }
        });
        let body = match serde_json::to_vec(&payload) {
            Ok(body) => body,
            Err(error) => return BenchReply::Absent(error.to_string()),
        };
        let response = match http(
            port,
            "POST",
            "/api/chat",
            Some(&body),
            Duration::from_secs(240),
        ) {
            Ok(response) => response,
            Err(text) => {
                stopped = text;
                break;
            }
        };
        let message = match chat_message(&response) {
            Ok(message) => message,
            Err(text) => {
                stopped = text;
                break;
            }
        };
        let calls = tool_calls(&message);
        messages.push(message);
        if calls.is_empty() {
            let next = Phase::of(round + 1);
            if !reminded && round + 1 < MAX_ROUNDS {
                reminded = true;
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": format!("Answer with a tool call, not prose. {}", bench.phase_prompt(next))
                }));
                continue;
            }
            stopped = "The model answered without calling a tool.".to_string();
            break;
        }
        for (index, (name, args)) in calls.into_iter().enumerate() {
            let answer = if index < MAX_CALLS_PER_ROUND {
                bench.call(&name, &args)
            } else {
                format!(
                    "Not run: at most {} tool calls run per round. Call it again next round if you still need it.",
                    MAX_CALLS_PER_ROUND
                )
            };
            messages
                .push(serde_json::json!({"role": "tool", "tool_name": name, "content": answer}));
            if bench.finished {
                break;
            }
        }
        if bench.finished {
            stopped = if bench.steps.last().is_some_and(|step| step.tool == "finish") {
                "The model finished.".to_string()
            } else {
                format!("It used all {} tool calls.", MAX_CALLS)
            };
            break;
        }
    }
    BenchReply::Done {
        model,
        digest,
        bench: Box::new(bench),
        stopped,
    }
}

/// The first local model, in preference order, that Ollama says can call tools.
fn choose_tool_model(port: u16, names: &[String]) -> Option<String> {
    for name in candidates(names).into_iter().take(8) {
        let body = serde_json::to_vec(&serde_json::json!({"model": name})).ok()?;
        let Ok(shown) = http(
            port,
            "POST",
            "/api/show",
            Some(&body),
            Duration::from_secs(10),
        ) else {
            continue;
        };
        if can_call_tools(&shown) {
            return Some(name);
        }
    }
    None
}

/// Local, non-embedding models, the known tool callers first.
pub(crate) fn candidates(names: &[String]) -> Vec<String> {
    let local: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|name| !is_cloud(name) && !name.to_ascii_lowercase().contains("embed"))
        .collect();
    const PREFER: &[&str] = &[
        "qwen3:14b",
        "qwen2.5:14b",
        "qwen3:8b",
        "qwen2.5:7b",
        "llama3.1:8b",
        "hermes3:8b",
    ];
    let mut out: Vec<String> = PREFER
        .iter()
        .filter(|want| local.contains(want))
        .map(|want| (*want).to_string())
        .collect();
    for name in local {
        if !out.iter().any(|known| known == name) {
            out.push(name.to_string());
        }
    }
    out
}

pub(crate) fn can_call_tools(show: &str) -> bool {
    serde_json::from_str::<Value>(show)
        .ok()
        .and_then(|value| value.get("capabilities").cloned())
        .and_then(|caps| caps.as_array().cloned())
        .is_some_and(|caps| caps.iter().any(|cap| cap.as_str() == Some("tools")))
}

pub(crate) fn chat_message(body: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(body).map_err(|error| error.to_string())?;
    if let Some(error) = value.get("error").and_then(Value::as_str) {
        return Err(error.to_string());
    }
    value
        .get("message")
        .cloned()
        .ok_or_else(|| "The local model returned no message.".to_string())
}

/// The calls in an assistant message. Arguments may arrive as an object or as JSON text.
pub(crate) fn tool_calls(message: &Value) -> Vec<(String, Value)> {
    message
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|calls| {
            calls
                .iter()
                .filter_map(|call| {
                    let function = call.get("function")?;
                    let name = function.get("name")?.as_str()?.to_string();
                    let args = match function.get("arguments") {
                        Some(Value::String(text)) => {
                            serde_json::from_str(text).unwrap_or(Value::Null)
                        }
                        Some(other) => other.clone(),
                        None => Value::Null,
                    };
                    Some((name, args))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn http(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<String, String> {
    let (status, bytes) = exchange(port, method, path, body, timeout)?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if !(200..300).contains(&status) {
        let detail = text.trim();
        if detail.is_empty() {
            return Err(format!("The local model returned HTTP {status}."));
        }
        return Err(format!("The local model returned HTTP {status}: {detail}"));
    }
    Ok(text)
}

fn exchange(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<(u16, Vec<u8>), String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, timeout)
        .map_err(|_| "The local model is not running.".to_string())?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nAccept: application/json\r\n"
    );
    if let Some(body) = body {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string())?;
    if let Some(body) = body {
        stream.write_all(body).map_err(|error| error.to_string())?;
    }
    let mut bytes = Vec::new();
    match stream.read_to_end(&mut bytes) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::TimedOut && !bytes.is_empty() => {}
        Err(error) => return Err(error.to_string()),
    }
    split_http(&bytes)
}

fn split_http(bytes: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let split = bytes
        .windows(4)
        .position(|mark| mark == b"\r\n\r\n")
        .ok_or_else(|| "The local model sent an incomplete response.".to_string())?;
    let header = std::str::from_utf8(&bytes[..split])
        .map_err(|_| "The local model sent a bad header.".to_string())?;
    let mut lines = header.lines();
    let status_line = lines.next().unwrap_or("");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| "The local model sent no status.".to_string())?;
    let mut length = None;
    let mut chunked = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            length = value.trim().parse::<usize>().ok();
        }
        if name.eq_ignore_ascii_case("transfer-encoding")
            && value.to_ascii_lowercase().contains("chunked")
        {
            chunked = true;
        }
    }
    let rest = &bytes[split + 4..];
    let body = if chunked {
        decode_chunks(rest)?
    } else if let Some(length) = length {
        rest.get(..length)
            .ok_or_else(|| "The local model sent a short body.".to_string())?
            .to_vec()
    } else {
        rest.to_vec()
    };
    Ok((status, body))
}

fn decode_chunks(mut rest: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let split = rest
            .windows(2)
            .position(|mark| mark == b"\r\n")
            .ok_or_else(|| "The local model sent a short chunk.".to_string())?;
        let line = std::str::from_utf8(&rest[..split])
            .map_err(|_| "The local model sent a bad chunk.".to_string())?;
        let size = usize::from_str_radix(line.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| "The local model sent a bad chunk.".to_string())?;
        rest = &rest[split + 2..];
        if size == 0 {
            break;
        }
        if rest.len() < size + 2 {
            return Err("The local model sent a short chunk.".to_string());
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
    Ok(out)
}

/// Ollama's digest for `name` in a tag list.
pub(crate) fn model_digest(body: &str, name: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value
        .get("models")?
        .as_array()?
        .iter()
        .find(|model| model.get("name").and_then(Value::as_str) == Some(name))?
        .get("digest")?
        .as_str()
        .map(str::to_string)
}

pub(crate) fn local_model_names(body: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    value
        .get("models")
        .and_then(Value::as_array)
        .map(|models| {
            models
                .iter()
                .filter_map(|model| model.get("name").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn is_cloud(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.split([':', '/']).any(|part| part == "cloud")
        || lower.ends_with("-cloud")
        || lower.contains("-cloud:")
}

#[cfg(test)]
mod tests {
    use super::{
        BenchReply, can_call_tools, candidates, chat_message, decode_chunks, exchange,
        local_model_names, model_digest, split_http, start_bench_on, tool_calls,
    };
    use crate::session::{MAX_CALLS, Workbench};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    #[test]
    fn cloud_and_embedding_tags_are_not_candidates() {
        let names = vec![
            "nemotron-3-ultra:cloud".to_string(),
            "nomic-embed-text:latest".to_string(),
            "muse:30b".to_string(),
            "llama3.1:8b".to_string(),
            "qwen2.5:14b".to_string(),
        ];
        assert_eq!(
            candidates(&names),
            vec!["qwen2.5:14b", "llama3.1:8b", "muse:30b"]
        );
        assert!(candidates(&["glm-5.2:cloud".to_string()]).is_empty());
        assert!(can_call_tools(r#"{"capabilities":["completion","tools"]}"#));
        assert!(!can_call_tools(r#"{"capabilities":["completion"]}"#));
    }

    #[test]
    fn tags_messages_and_calls_parse() {
        let names = local_model_names(r#"{"models":[{"name":"qwen3:8b"},{"name":"x:cloud"}]}"#);
        assert_eq!(names, vec!["qwen3:8b".to_string(), "x:cloud".to_string()]);
        let message = chat_message(
            r#"{"message":{"role":"assistant","content":"","tool_calls":[
                {"function":{"name":"measure","arguments":{"formula":"low"}}},
                {"function":{"name":"finish","arguments":"{\"note\":\"done\"}"}}]}}"#,
        )
        .unwrap();
        let calls = tool_calls(&message);
        assert_eq!(
            calls[0],
            ("measure".to_string(), serde_json::json!({"formula": "low"}))
        );
        assert_eq!(
            calls[1],
            ("finish".to_string(), serde_json::json!({"note": "done"}))
        );
        assert!(chat_message(r#"{"error":"missing"}"#).is_err());
    }

    #[test]
    fn chunked_and_length_bodies_split() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        let (status, body) = split_http(raw).unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, b"{}");
        assert_eq!(decode_chunks(b"5\r\nhello\r\n0\r\n\r\n").unwrap(), b"hello");
        let chunked =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
        let (_, body) = split_http(chunked).unwrap();
        assert_eq!(body, b"hello");
    }

    #[test]
    fn exchange_reads_a_loopback_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0_u8; 2048];
            let _ = stream.read(&mut buf);
            let body = br#"{"models":[]}"#;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(body).unwrap();
        });
        let (status, body) =
            exchange(port, "GET", "/api/tags", None, Duration::from_secs(3)).unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, br#"{"models":[]}"#);
        server.join().unwrap();
    }

    /// A fake Ollama: one scripted body per request, in order.
    fn serve(bodies: Vec<String>) -> (u16, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for body in bodies {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buf = [0_u8; 65536];
                loop {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    request.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&request).to_string();
                    if let Some(split) = text.find("\r\n\r\n") {
                        let length = text
                            .lines()
                            .find_map(|line| line.strip_prefix("Content-Length: "))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if request.len() >= split + 4 + length {
                            break;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                }
                seen.push(String::from_utf8_lossy(&request).to_string());
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(header.as_bytes()).unwrap();
                stream.write_all(body.as_bytes()).unwrap();
            }
            seen
        });
        (port, handle)
    }

    fn bench() -> Workbench {
        Workbench::new(
            crate::testing::seeds_board(),
            Vec::new(),
            Vec::new(),
            "2026-10-06",
        )
    }

    fn absent(reply: BenchReply) -> String {
        match reply {
            BenchReply::Absent(text) => text,
            BenchReply::Done { stopped, .. } => panic!("expected no session, got: {stopped}"),
        }
    }

    fn stopped(reply: BenchReply) -> (String, usize) {
        match reply {
            BenchReply::Done { stopped, bench, .. } => (stopped, bench.steps.len()),
            BenchReply::Absent(text) => panic!("expected a session, got: {text}"),
        }
    }

    #[test]
    fn a_session_says_why_it_could_not_start_or_had_to_stop() {
        let wait = Duration::from_secs(20);
        // Ollama is not running: nothing listens on a port just freed.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        assert!(
            absent(start_bench_on(port, bench()).recv_timeout(wait).unwrap())
                .contains("not running")
        );

        // Only models that cannot call tools.
        let (port, server) = serve(vec![
            r#"{"models":[{"name":"qwen2.5:7b"}]}"#.to_string(),
            r#"{"capabilities":["completion"]}"#.to_string(),
        ]);
        let text = absent(start_bench_on(port, bench()).recv_timeout(wait).unwrap());
        assert!(text.contains("can call tools"), "{text}");
        server.join().unwrap();

        // The model answers in prose without calling a tool.
        let (port, server) = serve(vec![
            r#"{"models":[{"name":"qwen3:14b"}]}"#.to_string(),
            r#"{"capabilities":["tools"]}"#.to_string(),
            r#"{"message":{"role":"assistant","content":"I think rank matters."}}"#.to_string(),
            r#"{"message":{"role":"assistant","content":"Rank matters, I said."}}"#.to_string(),
        ]);
        let (why, calls) = stopped(start_bench_on(port, bench()).recv_timeout(wait).unwrap());
        assert_eq!(why, "The model answered without calling a tool.");
        assert_eq!(calls, 0);
        let seen = server.join().unwrap();
        assert!(seen[3].contains("Answer with a tool call, not prose."));

        // The server reports an error mid-session.
        let (port, server) = serve(vec![
            r#"{"models":[{"name":"qwen3:14b"}]}"#.to_string(),
            r#"{"capabilities":["tools"]}"#.to_string(),
            r#"{"error":"model is loading"}"#.to_string(),
        ]);
        let (why, _) = stopped(start_bench_on(port, bench()).recv_timeout(wait).unwrap());
        assert_eq!(why, "model is loading");
        server.join().unwrap();
    }

    #[test]
    fn a_round_runs_at_most_three_calls_and_the_session_stops_at_its_budget() {
        let mut bodies = vec![
            r#"{"models":[{"name":"qwen3:14b"}]}"#.to_string(),
            r#"{"capabilities":["tools"]}"#.to_string(),
        ];
        let five: Vec<String> = (0..5)
            .map(|_| r#"{"function":{"name":"measure","arguments":{"formula":"low"}}}"#.to_string())
            .collect();
        // Rounds of five calls run three each: ten calls are spent in four rounds.
        for _ in 0..4 {
            bodies.push(format!(
                r#"{{"message":{{"role":"assistant","content":"","tool_calls":[{}]}}}}"#,
                five.join(",")
            ));
        }
        let (port, server) = serve(bodies);
        let reply = start_bench_on(port, bench())
            .recv_timeout(Duration::from_secs(20))
            .unwrap();
        let (why, calls) = stopped(reply);
        assert_eq!(why, format!("It used all {} tool calls.", MAX_CALLS));
        assert_eq!(calls, MAX_CALLS);
        let seen = server.join().unwrap();
        assert!(seen[3].contains("Not run: at most 3 tool calls run per round."));
    }

    #[test]
    fn a_workbench_session_runs_the_calls_and_stops_at_finish() {
        let (port, server) = serve(vec![
            r#"{"models":[{"name":"x:cloud"},{"name":"qwen3:14b","digest":"abc123"}]}"#.to_string(),
            r#"{"capabilities":["completion","tools"]}"#.to_string(),
            r#"{"message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"measure","arguments":{"formula":"low"}}}]}}"#.to_string(),
            r#"{"message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"finish","arguments":{"note":"Only the seed changed, so no knob can be weighed."}}}]}}"#.to_string(),
        ]);
        let reply = start_bench_on(port, bench())
            .recv_timeout(Duration::from_secs(20))
            .unwrap();
        let BenchReply::Done {
            model,
            digest,
            bench,
            stopped,
        } = reply
        else {
            panic!("the session should finish");
        };
        assert_eq!(model, "qwen3:14b");
        assert_eq!(digest.as_deref(), Some("abc123"));
        assert_eq!(stopped, "The model finished.");
        assert_eq!(bench.steps.len(), 2);
        assert!(bench.steps[0].result.contains("seed noise"));
        assert_eq!(
            bench.note.as_deref(),
            Some("Only the seed changed, so no knob can be weighed.")
        );
        let seen = server.join().unwrap();
        assert!(seen[2].contains("\"tools\""));
        assert!(seen[3].contains("\"role\":\"tool\""));
        assert!(
            !seen.iter().any(|request| request.contains("x:cloud\""))
                || seen[1].contains("qwen3:14b")
        );
    }

    #[test]
    fn a_digest_is_read_for_the_chosen_model_only() {
        let tags = r#"{"models":[{"name":"a:1","digest":"d1"},{"name":"b:2"}]}"#;
        assert_eq!(model_digest(tags, "a:1").as_deref(), Some("d1"));
        assert_eq!(model_digest(tags, "b:2"), None);
        assert_eq!(model_digest(tags, "c:3"), None);
        assert_eq!(model_digest("not json", "a:1"), None);
    }
}
