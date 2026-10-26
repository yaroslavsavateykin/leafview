//! Bounded subprocess output without pipe deadlocks. Every child is killed and reaped.
use anyhow::{bail, Context, Result};
use std::{
    fs::File,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tempfile::tempfile;

struct ChildGuard(std::process::Child);

// Compiler subprocesses share a dedicated process group (including helpers).
struct CompileGuard(std::process::Child);
impl Drop for CompileGuard {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .args(["-KILL", "--"])
            .arg(format!("-{}", self.0.id()))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn run_compile(command: &mut Command, cancelled: &dyn Fn() -> bool) -> Result<()> {
    use std::{
        io::{Read, Seek},
        os::unix::process::CommandExt,
    };
    let mut output = tempfile()?;
    let mut limited = Command::new("prlimit");
    limited
        .args(["--as=3221225472", "--fsize=94371840", "--cpu=60", "--"])
        .arg(command.get_program())
        .args(command.get_args())
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output.try_clone()?)
        .env("LC_ALL", "C")
        .process_group(0);
    if let Some(dir) = command.get_current_dir() {
        limited.current_dir(dir);
    }
    let mut child = CompileGuard(limited.spawn().context("start Tectonic")?);
    let start = Instant::now();
    loop {
        if cancelled() {
            bail!("compilation cancelled");
        }
        if start.elapsed() > Duration::from_secs(60) {
            bail!("Tectonic timed out (60s)");
        }
        if output.metadata()?.len() > 1024 * 1024 {
            bail!("Tectonic output exceeds 1 MiB");
        }
        if let Some(status) = child.0.try_wait()? {
            if status.success() {
                return Ok(());
            }
            output.rewind()?;
            let mut text = String::new();
            output.take(65536).read_to_string(&mut text)?;
            bail!(
                "Tectonic failed ({status}): {}",
                text.lines()
                    .rev()
                    .take(12)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn run(command: &mut Command, limit: u64, cancelled: impl Fn() -> bool) -> Result<File> {
    let out = tempfile()?;
    let err = tempfile()?;
    // util-linux imposes kernel limits before executing Poppler (no pre_exec/unsafe).
    let mut limited = Command::new("prlimit");
    limited
        .args(["--as=536870912", "--fsize=94371840", "--cpu=12", "--"])
        .arg(command.get_program())
        .args(command.get_args());
    limited
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(out.try_clone()?)
        .stderr(err.try_clone()?);
    let mut guard = ChildGuard(
        limited
            .spawn()
            .context("start Poppler (install poppler-utils and util-linux)")?,
    );
    let child = &mut guard.0;
    let started = Instant::now();
    loop {
        let problem = if cancelled() {
            Some("cancelled")
        } else if started.elapsed() > Duration::from_secs(12) {
            Some("Poppler timed out")
        } else if out.metadata()?.len() > limit || err.metadata()?.len() > 65536 {
            Some("Poppler output exceeds limit")
        } else {
            None
        };
        if let Some(problem) = problem {
            let _ = child.kill();
            let _ = child.wait();
            bail!("{problem}");
        }
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                bail!("Poppler failed ({status}); PDF may be damaged or encrypted");
            }
            return Ok(out);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_reaps_child() {
        let start = Instant::now();
        assert!(run(Command::new("sleep").arg("10"), 10, || true).is_err());
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn compiler_cancellation_is_prompt() {
        let start = Instant::now();
        assert!(run_compile(Command::new("sleep").arg("30"), &|| true).is_err());
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn compiler_preserves_working_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("present"), "ok").unwrap();
        run_compile(
            Command::new("test")
                .arg("-f")
                .arg("present")
                .current_dir(dir.path()),
            &|| false,
        )
        .unwrap();
    }
    #[test]
    fn output_is_bounded() {
        assert!(run(&mut Command::new("yes"), 1024, || false).is_err());
    }
}
