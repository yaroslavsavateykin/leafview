use anyhow::{Context, Result};
use helix_core::Rope;
use std::{
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
pub enum Task {
    Open(PathBuf),
    Save(PathBuf, u64, Rope),
    Format(PathBuf, u64, Rope),
    Lsp(PathBuf, u64, Rope, usize, String),
}
pub enum Reply {
    Opened(PathBuf, Rope),
    Saved(PathBuf, u64),
    Formatted(PathBuf, u64, String),
    Lsp(PathBuf, u64, String),
}
#[derive(Default)]
pub struct Runner {
    client: Option<(PathBuf, u64, super::lsp::Client)>,
    pub cancel: std::sync::Arc<AtomicBool>,
}
impl Runner {
    pub fn run(&mut self, task: Task, stop: &AtomicBool) -> Result<Reply> {
        if let Task::Lsp(path, version, text, cursor, command) = task {
            let source_flags = command
                .strip_prefix("complete:")
                .unwrap_or("lsp,path")
                .to_owned();
            let command = if command.starts_with("complete:") {
                "complete".to_owned()
            } else {
                command
            };
            let paths = if command == "complete" && source_flags.split(',').any(|s| s == "path") {
                super::completion_sources::paths(&path, &text, cursor)
            } else {
                vec![]
            };
            if command == "complete"
                && (!source_flags.split(',').any(|s| s == "lsp") || !paths.is_empty())
            {
                return Ok(Reply::Lsp(
                    path,
                    version,
                    serde_json::json!([{"method":"textDocument/completion","result":paths}])
                        .to_string(),
                ));
            }
            if command == "lsp-restart" {
                self.client.take();
                return Ok(Reply::Lsp(
                    path,
                    version,
                    "LSP will restart on next request".into(),
                ));
            }
            let extension = path.extension().and_then(|x| x.to_str()).unwrap_or("");
            let (language, executable, args): (&str, &str, Vec<String>) = match extension {
                "typ" => ("typst", "tinymist", vec![]),
                "rs" => ("rust", "rust-analyzer", vec![]),
                "c" | "h" | "cpp" | "hpp" => ("cpp", "clangd", vec![]),
                "py" => ("python", "basedpyright-langserver", vec!["--stdio".into()]),
                "js" | "jsx" => (
                    "javascript",
                    "typescript-language-server",
                    vec!["--stdio".into()],
                ),
                "ts" | "tsx" => (
                    "typescript",
                    "typescript-language-server",
                    vec!["--stdio".into()],
                ),
                _ => (extension, "", vec![]),
            };
            if self.client.as_ref().is_none_or(|(p, _, _)| p != &path) {
                let mut command: Vec<String> = std::env::var("LEAFVIEW_LSP")
                    .ok()
                    .map(|v| v.split_whitespace().map(str::to_owned).collect())
                    .unwrap_or_else(|| {
                        let mut v = vec![executable.into()];
                        v.extend(args);
                        v
                    });
                anyhow::ensure!(
                    !command.is_empty() && !command[0].is_empty(),
                    "No language server; set LEAFVIEW_LSP"
                );
                let mut client = super::lsp::Client::start(&path, std::mem::take(&mut command))?;
                let started = Instant::now();
                while !client.ready {
                    anyhow::ensure!(
                        !stop.load(Ordering::Relaxed)
                            && started.elapsed() < Duration::from_secs(15),
                        "LSP initialization timed out"
                    );
                    client.poll()?;
                    std::thread::sleep(Duration::from_millis(10));
                }
                client.open(&text, version, language)?;
                self.client = Some((path.clone(), version, client));
            }
            let (_, previous, client) = self.client.as_mut().unwrap();
            if *previous != version {
                client.change(&text, version)?;
                *previous = version;
            }
            client.messages.clear();
            let resolve = command
                .strip_prefix("resolve:")
                .map(serde_json::from_str::<serde_json::Value>)
                .transpose()?;
            let method = match command.as_str() {
                _ if resolve.is_some() => "completionItem/resolve",
                "hover" => "textDocument/hover",
                "complete" => "textDocument/completion",
                "definition" => "textDocument/definition",
                "references" => "textDocument/references",
                "signature" => "textDocument/signatureHelp",
                "symbols" => "textDocument/documentSymbol",
                "diagnostics" => "",
                _ => anyhow::bail!("Unsupported LSP request"),
            };
            if !method.is_empty() {
                let extra = if command == "references" {
                    serde_json::json!({"context":{"includeDeclaration":true}})
                } else if command == "complete" {
                    serde_json::json!({"context":{"triggerKind":1}})
                } else if command == "signature" {
                    serde_json::json!({"context":{"triggerKind":3,"isRetrigger":true}})
                } else {
                    serde_json::json!({})
                };
                if let Some(item) = resolve {
                    client.request(method, item)?;
                } else {
                    client.position_request(method, &text, cursor, extra)?;
                }
            }
            let started = Instant::now();
            loop {
                if self.cancel.swap(false, Ordering::Relaxed) || stop.load(Ordering::Relaxed) {
                    client.cancel_pending();
                    anyhow::bail!("LSP request cancelled");
                }
                client.poll()?;
                if client.messages.iter().any(|reply| {
                    reply["method"] == method
                        || (method.is_empty()
                            && reply["method"] == "textDocument/publishDiagnostics")
                }) {
                    break;
                }
                anyhow::ensure!(
                    !stop.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(10),
                    "LSP request timed out"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            let messages: Vec<_> = std::mem::take(&mut client.messages)
                .into_iter()
                .filter(|reply| reply["method"] == method || method.is_empty())
                .collect();
            return Ok(Reply::Lsp(path, version, serde_json::to_string(&messages)?));
        }
        let reply = run(task, stop)?;
        if let Reply::Saved(path, _) = &reply {
            if let Some((open, _, client)) = &self.client {
                if path == open {
                    let _ = client.save();
                }
            }
        }
        Ok(reply)
    }
}
pub fn run(task: Task, stop: &AtomicBool) -> Result<Reply> {
    match task {
        Task::Lsp(..) => anyhow::bail!("LSP runner required"),
        Task::Open(path) => {
            let file = std::fs::File::open(&path)?;
            anyhow::ensure!(
                file.metadata()?.len() <= 256 * 1024 * 1024,
                "source exceeds 256 MiB"
            );
            let rope = Rope::from_reader(std::io::BufReader::new(file))?;
            Ok(Reply::Opened(path, rope))
        }
        Task::Save(path, version, text) => {
            super::buffer::atomic_save(&path, &text)?;
            Ok(Reply::Saved(path, version))
        }
        Task::Format(path, version, text) => {
            let extension = path.extension().and_then(|x| x.to_str()).unwrap_or("");
            let (program, args): (&str, Vec<String>) = match extension {
                "typ" => ("typstyle", vec![]),
                "rs" => ("rustfmt", vec!["--emit".into(), "stdout".into()]),
                "py" => ("black", vec!["--quiet".into(), "-".into()]),
                "c" | "h" | "cpp" | "hpp" => ("clang-format", vec![]),
                "js" | "ts" | "tsx" | "jsx" | "json" => (
                    "prettier",
                    vec!["--stdin-filepath".into(), path.display().to_string()],
                ),
                _ => anyhow::bail!("No formatter configured for {extension}"),
            };
            let override_command = std::env::var("LEAFVIEW_FORMATTER").ok();
            let override_args: Vec<_> = override_command
                .as_deref()
                .unwrap_or("")
                .split_whitespace()
                .collect();
            let (program, args) = if let Some(program) = override_args.first() {
                (
                    *program,
                    override_args[1..].iter().map(|s| s.to_string()).collect(),
                )
            } else {
                (program, args)
            };
            let mut input = tempfile::tempfile()?;
            for chunk in text.chunks() {
                input.write_all(chunk.as_bytes())?;
            }
            input.seek(SeekFrom::Start(0))?;
            let output = tempfile::tempfile()?;
            let errors = tempfile::tempfile()?;
            let mut child = Command::new(program)
                .args(args)
                .current_dir(path.parent().unwrap_or(std::path::Path::new(".")))
                .stdin(input)
                .stdout(output.try_clone()?)
                .stderr(errors.try_clone()?)
                .spawn()
                .with_context(|| format!("start {program}"))?;
            let started = Instant::now();
            loop {
                if stop.load(Ordering::Relaxed)
                    || started.elapsed() > Duration::from_secs(15)
                    || output.metadata()?.len() > 256 * 1024 * 1024
                {
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!("formatter cancelled, oversized or timed out");
                }
                if let Some(status) = child.try_wait()? {
                    if !status.success() {
                        let mut errors = errors;
                        errors.seek(SeekFrom::Start(0))?;
                        let mut detail = String::new();
                        errors.take(8192).read_to_string(&mut detail)?;
                        anyhow::bail!(
                            "{program} failed: {status}: {}",
                            crate::links::sanitize(&detail)
                        );
                    }
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let mut output = output;
            output.seek(SeekFrom::Start(0))?;
            let mut formatted = String::new();
            output.read_to_string(&mut formatted)?;
            Ok(Reply::Formatted(path, version, formatted))
        }
    }
}
