//! Owned incremental Typst helper process.
use crate::live::{reply_path, CompileReply, CompileRequest};
use anyhow::{Context, Result};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    time::{Duration, Instant},
};
pub struct CompilerProcess {
    child: Child,
    input: ChildStdin,
    directory: tempfile::TempDir,
}
impl Drop for CompilerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl CompilerProcess {
    pub fn new(input: &Path, root: &Path, options: &crate::typst_options::Options) -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let mut child = Command::new("prlimit")
            .args(["--as=3221225472", "--"])
            .arg(std::env::current_exe()?)
            .arg("--typst-compiler")
            .arg(input)
            .arg(root)
            .arg(serde_json::to_string(options)?)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let input = child.stdin.take().context("compiler stdin")?;
        Ok(Self {
            child,
            input,
            directory,
        })
    }
    pub fn compile(
        &mut self,
        generation: u64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(CompileReply, PathBuf)> {
        let output = self.directory.path().join("document.pdf");
        let reply = reply_path(&output);
        if reply.exists() {
            std::fs::remove_file(&reply)?;
        }
        serde_json::to_writer(
            &mut self.input,
            &CompileRequest {
                generation,
                output: output.clone(),
            },
        )?;
        self.input.write_all(b"\n")?;
        self.input.flush()?;
        let start = Instant::now();
        while !reply.exists() {
            if cancelled() || start.elapsed() > Duration::from_secs(60) {
                let _ = self.child.kill();
                anyhow::bail!("Typst compilation cancelled or timed out");
            }
            anyhow::ensure!(
                self.child.try_wait()?.is_none(),
                "Typst compiler stopped (resource limit or internal error)"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        anyhow::ensure!(
            std::fs::metadata(&reply)?.len() <= 96 * 1024 * 1024,
            "source map exceeds 96 MiB"
        );
        let result = serde_json::from_reader(std::fs::File::open(reply)?)?;
        Ok((result, output))
    }
}
