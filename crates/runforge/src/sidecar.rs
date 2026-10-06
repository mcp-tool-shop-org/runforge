//! The in-window sidecar. It talks only to a local Ollama on 127.0.0.1.
//!
//! Cloud tags are refused. This module does not press Train.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::Value;

pub enum SidecarReply {
    Answer { model: String, text: String },
    Absent(String),
}

pub fn start_ask(prompt: String) -> std::sync::mpsc::Receiver<SidecarReply> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("runforge-sidecar".to_string())
        .spawn(move || {
            let reply = match run_ask(&prompt) {
                Ok((model, text)) => SidecarReply::Answer { model, text },
                Err(text) => SidecarReply::Absent(text),
            };
            let _ = tx.send(reply);
        });
    rx
}

fn run_ask(prompt: &str) -> Result<(String, String), String> {
    let tags = http(11434, "GET", "/api/tags", None, Duration::from_secs(4))?;
    let names = local_model_names(&tags);
    let Some(model) = choose_model(&names) else {
        return Err("No local model is available. Cloud tags are not used.".to_string());
    };
    let payload = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": { "temperature": 0.2, "num_predict": 700 }
    });
    let bytes = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
    let response = http(
        11434,
        "POST",
        "/api/generate",
        Some(&bytes),
        Duration::from_secs(180),
    )?;
    let text = generate_text(&response)?;
    Ok((model, text))
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

pub(crate) fn choose_model(names: &[String]) -> Option<String> {
    let local: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|name| !is_cloud(name) && !name.to_ascii_lowercase().contains("embed"))
        .collect();
    const PREFER: &[&str] = &["qwen2.5:14b", "qwen3:8b", "qwen2.5:7b", "llama3.1:8b"];
    for want in PREFER {
        if local.contains(want) {
            return Some((*want).to_string());
        }
    }
    local.first().map(|name| (*name).to_string())
}

fn is_cloud(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.split([':', '/']).any(|part| part == "cloud")
        || lower.ends_with("-cloud")
        || lower.contains("-cloud:")
}

pub(crate) fn generate_text(body: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(body).map_err(|error| error.to_string())?;
    if let Some(error) = value.get("error").and_then(Value::as_str) {
        return Err(error.to_string());
    }
    value
        .get("response")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "The local model returned an empty answer.".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        choose_model, decode_chunks, exchange, generate_text, local_model_names, split_http,
    };
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    #[test]
    fn cloud_tags_are_not_chosen() {
        let names = vec![
            "nemotron-3-ultra:cloud".to_string(),
            "nomic-embed-text:latest".to_string(),
            "llama3.1:8b".to_string(),
            "qwen2.5:14b".to_string(),
            "translategemma:27b".to_string(),
        ];
        assert_eq!(choose_model(&names).as_deref(), Some("qwen2.5:14b"));
        assert_eq!(
            choose_model(&["foo-cloud:latest".to_string(), "hermes3:8b".to_string()]).as_deref(),
            Some("hermes3:8b")
        );
        assert!(choose_model(&["nomic-embed-text:latest".to_string()]).is_none());
        assert!(choose_model(&["glm-5.2:cloud".to_string()]).is_none());
    }

    #[test]
    fn tags_and_answers_parse() {
        let names = local_model_names(r#"{"models":[{"name":"qwen3:8b"},{"name":"x:cloud"}]}"#);
        assert_eq!(names, vec!["qwen3:8b".to_string(), "x:cloud".to_string()]);
        assert_eq!(
            generate_text(r#"{"response":" Hold the recipe. "}"#).unwrap(),
            "Hold the recipe."
        );
        assert!(generate_text(r#"{"error":"missing"}"#).is_err());
        assert!(generate_text(r#"{"response":"  "}"#).is_err());
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
}
