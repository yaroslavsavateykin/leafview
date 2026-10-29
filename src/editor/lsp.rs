//! Bounded stdio JSON-RPC client for external language servers; no UI-thread IO.
use anyhow::{Context, Result};
use helix_core::Rope;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
pub struct Client {
    child: Child,
    input: Option<mpsc::SyncSender<Value>>,
    output: mpsc::Receiver<Value>,
    threads: Vec<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    next: u64,
    pub ready: bool,
    pub messages: Vec<Value>,
    pub version: u64,
    uri: String,
    pending: HashMap<u64, (String, u64, Instant)>,
}
fn uri(path: &Path) -> String {
    let mut out = "file://".to_owned();
    for byte in path.as_os_str().as_encoded_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-_.~".contains(byte) {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
pub fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                length = Some(value.trim().parse::<usize>()?);
            }
        }
        anyhow::ensure!(line.len() < 8192, "LSP header too long");
    }
    let length = length.context("missing Content-Length")?;
    anyhow::ensure!(length <= 16 * 1024 * 1024, "LSP message exceeds 16 MiB");
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}
impl Client {
    pub fn start(path: &Path, command: Vec<String>) -> Result<Self> {
        let mut child = Command::new(command.first().context("empty LSP command")?)
            .args(&command[1..])
            .current_dir(path.parent().unwrap_or(Path::new(".")))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (input, rx) = mpsc::sync_channel::<Value>(64);
        let (tx, output) = mpsc::sync_channel(64);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let writer = std::thread::spawn(move || {
            while let Ok(message) = rx.recv() {
                if stopped.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(data) = serde_json::to_vec(&message) else {
                    break;
                };
                if write!(stdin, "Content-Length: {}\r\n\r\n", data.len())
                    .and_then(|_| stdin.write_all(&data))
                    .and_then(|_| stdin.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        let stopped = stop.clone();
        let reader = std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Ok(Some(mut value)) = read_message(&mut reader) {
                loop {
                    if stopped.load(Ordering::Relaxed) {
                        return;
                    }
                    match tx.try_send(value) {
                        Ok(()) => break,
                        Err(mpsc::TrySendError::Full(v)) => {
                            value = v;
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => return,
                    }
                }
            }
        });
        let mut client = Self {
            child,
            input: Some(input),
            output,
            threads: vec![writer, reader],
            stop,
            next: 1,
            ready: false,
            messages: vec![],
            version: 0,
            uri: uri(path),
            pending: HashMap::new(),
        };
        client.request("initialize",json!({"processId":std::process::id(),"rootUri":uri(path.parent().unwrap()),"capabilities":{"general":{"positionEncodings":["utf-16"]},"textDocument":{"synchronization":{"didSave":true},"completion":{"completionItem":{"snippetSupport":false}}}}}))?;
        Ok(client)
    }
    fn send(&self, value: Value) -> Result<()> {
        self.input
            .as_ref()
            .context("LSP stopped")?
            .try_send(value)
            .context("LSP queue full or stopped")
    }
    pub fn notify(&self, method: &str, params: Value) -> Result<()> {
        self.send(json!({"jsonrpc":"2.0","method":method,"params":params}))
    }
    pub fn request(&mut self, method: &str, params: Value) -> Result<u64> {
        let id = self.next;
        self.next += 1;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        self.pending
            .insert(id, (method.into(), self.version, Instant::now()));
        Ok(id)
    }
    pub fn open(&mut self, text: &Rope, version: u64, language: &str) -> Result<()> {
        self.version = version;
        self.notify("textDocument/didOpen",json!({"textDocument":{"uri":self.uri,"languageId":language,"version":version,"text":text.to_string()}}))
    }
    pub fn change(&mut self, text: &Rope, version: u64) -> Result<()> {
        self.version = version;
        self.notify("textDocument/didChange",json!({"textDocument":{"uri":self.uri,"version":version},"contentChanges":[{"text":text.to_string()}]}))
    }
    pub fn save(&self) -> Result<()> {
        self.notify(
            "textDocument/didSave",
            json!({"textDocument":{"uri":self.uri}}),
        )
    }
    pub fn position_request(
        &mut self,
        method: &str,
        text: &Rope,
        pos: usize,
        extra: Value,
    ) -> Result<()> {
        let line = text.char_to_line(pos);
        let start = text.line_to_char(line);
        let column = text
            .slice(start..pos)
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        let mut params =
            json!({"textDocument":{"uri":self.uri},"position":{"line":line,"character":column}});
        if let Some(extra) = extra.as_object() {
            for (k, v) in extra {
                params[k] = v.clone();
            }
        }
        self.request(method, params)?;
        Ok(())
    }
    pub fn poll(&mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            anyhow::bail!("LSP exited: {status}; :lsp-restart to retry");
        }
        anyhow::ensure!(
            !self
                .pending
                .values()
                .any(|(_, _, time)| time.elapsed() > Duration::from_secs(20)),
            "LSP request timed out; :lsp-restart to retry"
        );
        while let Ok(value) = self.output.try_recv() {
            if let Some(method) = value.get("method").and_then(Value::as_str) {
                if value.get("id").is_some() {
                    let id = value["id"].clone();
                    let result = if method == "workspace/configuration" {
                        json!([])
                    } else {
                        Value::Null
                    };
                    self.send(json!({"jsonrpc":"2.0","id":id,"result":result}))?;
                } else if method == "textDocument/publishDiagnostics"
                    && value["params"]["uri"] == self.uri
                {
                    if value["params"]
                        .get("version")
                        .and_then(Value::as_u64)
                        .is_none_or(|v| v == self.version)
                    {
                        self.messages.push(value);
                    }
                }
                continue;
            }
            if let Some(id) = value.get("id").and_then(Value::as_u64) {
                if let Some((method, version, _)) = self.pending.remove(&id) {
                    if method == "initialize" {
                        anyhow::ensure!(
                            value.get("error").is_none(),
                            "LSP initialize failed: {value}"
                        );
                        self.notify("initialized", json!({}))?;
                        self.ready = true;
                    } else if version == self.version {
                        self.messages.push(json!({"method":method,"version":version,"result":value["result"],"error":value["error"]}));
                    }
                }
            }
        }
        if self.messages.len() > 64 {
            self.messages.drain(..self.messages.len() - 64);
        }
        Ok(())
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.notify(
            "textDocument/didClose",
            json!({"textDocument":{"uri":self.uri}}),
        );
        self.stop.store(true, Ordering::Relaxed);
        self.input.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stdio_initialize_unicode_versions_and_exit() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("server.py");
        std::fs::write(&script, r#"import sys,json
while True:
 line=sys.stdin.buffer.readline()
 if not line: break
 length=int(line.split(b':')[1]); sys.stdin.buffer.readline()
 m=json.loads(sys.stdin.buffer.read(length))
 if 'id' not in m: continue
 result={'capabilities':{}} if m['method']=='initialize' else {'contents':'unicode λ'}
 data=json.dumps({'jsonrpc':'2.0','id':m['id'],'result':result}).encode()
 sys.stdout.buffer.write(b'Content-Length: '+str(len(data)).encode()+b'\r\n\r\n'+data);sys.stdout.buffer.flush()
"#).unwrap();
        let path = dir.path().join("file.typ");
        let mut client =
            Client::start(&path, vec!["python3".into(), script.display().to_string()]).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !client.ready {
            client.poll().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        client.open(&Rope::from_str("λ中\n"), 1, "typst").unwrap();
        client.change(&Rope::from_str("λ中x\n"), 2).unwrap();
        client
            .position_request(
                "textDocument/hover",
                &Rope::from_str("λ中x\n"),
                2,
                json!({}),
            )
            .unwrap();
        while client.messages.is_empty() {
            client.poll().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(client.messages[0]["version"], 2);
        assert_eq!(client.messages[0]["result"]["contents"], "unicode λ");
    }
    #[test]
    fn framing_and_bounds() {
        let value = json!({"text":"λ中"});
        let body = serde_json::to_vec(&value).unwrap();
        let mut bytes = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        bytes.extend(body);
        assert_eq!(
            read_message(&mut std::io::Cursor::new(bytes)).unwrap(),
            Some(value)
        );
        assert!(read_message(&mut std::io::Cursor::new(
            b"Content-Length: 999999999\r\n\r\n"
        ))
        .is_err());
    }
}
