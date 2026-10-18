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
    fn output_is_bounded() {
        assert!(run(&mut Command::new("yes"), 1024, || false).is_err());
    }
}
