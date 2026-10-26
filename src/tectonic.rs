//! External Tectonic backend. PDF and SyncTeX live in one private generation directory.
use crate::live::{changed_range, CompileReply, SourcePoint};
use anyhow::{Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

pub fn parse(args: impl Iterator<Item = OsString>) -> Result<Option<(PathBuf, bool)>> {
    let mut args = args.peekable();
    if args.peek().is_none_or(|s| s != "tectonic") {
        return Ok(None);
    }
    args.next();
    let mut input = None;
    let mut no_pdf = false;
    let mut literal = false;
    for arg in args {
        if !literal && arg == "--" {
            literal = true;
        } else if !literal && arg == "--no-pdf" {
            no_pdf = true;
        } else if !literal && (arg == "--help" || arg == "-h") {
            println!("Usage: leafview tectonic <file.tex> [--no-pdf]\nRequires tectonic, Poppler and prlimit. Install synctex for forward search.");
            std::process::exit(0);
        } else {
            anyhow::ensure!(
                literal || !arg.to_string_lossy().starts_with('-'),
                "unknown Tectonic option: {}",
                arg.to_string_lossy()
            );
            anyhow::ensure!(input.is_none(), "expected one LaTeX input file");
            input = Some(PathBuf::from(arg));
        }
    }
    Ok(Some((
        input.context("Usage: leafview tectonic <file.tex> [--no-pdf]")?,
        no_pdf,
    )))
}

pub struct Compiler {
    input: PathBuf,
    directory: Option<tempfile::TempDir>,
    dependencies: BTreeSet<PathBuf>,
}
impl Compiler {
    pub fn new(input: &Path) -> Result<Self> {
        Ok(Self {
            input: input.to_owned(),
            directory: None,
            dependencies: BTreeSet::from([input.to_owned()]),
        })
    }
    pub fn compile(
        &mut self,
        generation: u64,
        cancelled: &dyn Fn() -> bool,
        previous: &BTreeMap<PathBuf, String>,
    ) -> Result<(CompileReply, PathBuf)> {
        let start = Instant::now();
        let directory = tempfile::tempdir()?;
        let root = self.input.parent().context("input parent")?;
        // Capture known source bytes before, not after, the compiler reads them.
        let before = fingerprints(&self.dependencies);
        let known = self.dependencies.clone();
        let rules = directory.path().join("dependencies.mk");
        let output = directory
            .path()
            .join(self.input.file_name().unwrap())
            .with_extension("pdf");
        let result = crate::process::run_compile(
            Command::new("tectonic")
                .current_dir(root)
                .args(["-X", "compile"])
                .arg(&self.input)
                .args([
                    "--synctex",
                    "--keep-logs",
                    "--keep-intermediates",
                    "--untrusted",
                    "--outdir",
                ])
                .arg(directory.path())
                .arg("--makefile-rules")
                .arg(&rules),
            cancelled,
        );
        if let Ok(text) = std::fs::read_to_string(&rules) {
            self.dependencies
                .extend(make_dependencies(&text, root, directory.path()));
        }
        // Failed builds may not emit rules; the log still identifies opened TeX files.
        let log = std::fs::read_to_string(output.with_extension("log")).unwrap_or_default();
        for token in log.split(|c: char| c.is_whitespace() || c == '(' || c == ')') {
            if token.ends_with(".tex") || token.ends_with(".bib") {
                let p = root.join(token);
                if let Ok(p) = p.canonicalize() {
                    self.dependencies.insert(p);
                }
            }
        }
        let sources = snapshots(&self.dependencies);
        let after = fingerprints(&self.dependencies);
        let stale = self
            .dependencies
            .iter()
            .filter(|p| !known.contains(*p) || before.get(*p) != after.get(*p))
            .cloned()
            .collect();
        let mut points = Vec::new();
        let error = result
            .err()
            .map(|e| compile_diagnostic(&log, &self.input, &format!("{e:#}")));
        if error.is_none() {
            let search_start = Instant::now();
            let search_cancelled =
                || cancelled() || search_start.elapsed() > std::time::Duration::from_secs(3);
            for (file, new) in &sources {
                if file.extension().is_none_or(|s| s != "tex") {
                    continue;
                }
                if let Some(range) = previous.get(file).and_then(|old| changed_range(old, new)) {
                    if let Some(point) = forward(file, new, range, &output, &search_cancelled) {
                        points.push(point);
                    }
                }
            }
        }
        self.directory = Some(directory);
        Ok((
            CompileReply {
                generation,
                stale,
                synctex_coordinates: true,
                dependencies: self.dependencies.iter().cloned().collect(),
                sources,
                points,
                error,
                millis: start.elapsed().as_millis() as u64,
            },
            output,
        ))
    }
}
// Metadata covers binary assets too, including atomic replacement and removal.
fn fingerprints(
    paths: &BTreeSet<PathBuf>,
) -> BTreeMap<PathBuf, (u64, Option<std::time::SystemTime>, u64)> {
    use std::os::unix::fs::MetadataExt;
    paths
        .iter()
        .filter_map(|p| {
            let m = std::fs::metadata(p).ok()?;
            Some((p.clone(), (m.len(), m.modified().ok(), m.ino())))
        })
        .collect()
}
fn snapshots(paths: &BTreeSet<PathBuf>) -> BTreeMap<PathBuf, String> {
    let mut budget = 32 * 1024 * 1024;
    paths
        .iter()
        .filter_map(|p| {
            if !matches!(
                p.extension().and_then(|s| s.to_str()),
                Some("tex" | "bib" | "sty" | "cls")
            ) {
                return None;
            }
            let size = std::fs::metadata(p).ok()?.len() as usize;
            if size > budget {
                return None;
            }
            budget -= size;
            Some((p.clone(), std::fs::read_to_string(p).ok()?))
        })
        .collect()
}
/// Make escaping (including spaces, # and backslash-newline continuations).
fn make_dependencies(text: &str, root: &Path, outdir: &Path) -> BTreeSet<PathBuf> {
    // Tectonic 0.17 emits one *unescaped* path per continuation line,
    // and incorrectly roots relative dependencies at outdir. Handle its actual
    // dialect before the conventional escaped Make fallback.
    if text.contains("\\\n") {
        if let Some((_, rhs)) = text.split_once(':') {
            let paths: BTreeSet<_> = rhs
                .split("\\\n")
                .filter_map(|s| {
                    let path = PathBuf::from(s.trim());
                    let path = if let Ok(relative) = path.strip_prefix(outdir) {
                        root.join(relative)
                    } else {
                        root.join(path)
                    };
                    path.canonicalize().ok()
                })
                .collect();
            if !paths.is_empty() {
                return paths;
            }
        }
    }
    let text = text.replace("\\\n", "");
    let Some((_, rhs)) = text.split_once(':') else {
        return BTreeSet::new();
    };
    let mut words = Vec::new();
    let mut word = String::new();
    let mut escaped = false;
    for c in rhs.chars() {
        if escaped {
            word.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
        .into_iter()
        .filter_map(|p| root.join(p.replace("$$", "$")).canonicalize().ok())
        .collect()
}
fn compile_diagnostic(log: &str, input: &Path, error: &str) -> String {
    if error.contains("bundle isn't cached") {
        return format!("{}\nTectonic could not download its TeX bundle (not a LaTeX source error).\n\nRun outside LeafView to prepare the bundle and inspect the network error:\n  tectonic -X compile <your main.tex>\n\nCheck the network/proxy or configure a local Tectonic bundle, then press r.\n\nOriginal diagnostic:\n{error}", input.display());
    }
    format!("{}\n{error}", diagnostic(log, input))
}
fn diagnostic(log: &str, input: &Path) -> String {
    let lines: Vec<_> = log.lines().collect();
    let mut file = input.display().to_string();
    for (i, line) in lines.iter().enumerate() {
        if let Some(pos) = line.find('(') {
            let name = line[pos + 1..].split_whitespace().next().unwrap_or("");
            if name.ends_with(".tex") {
                file = name.to_owned();
            }
        }
        if line.starts_with('!') {
            let location = lines
                .iter()
                .skip(i + 1)
                .take(12)
                .find(|l| l.starts_with("l."))
                .copied()
                .unwrap_or("");
            return format!("{file}: {line}\n{location}");
        }
    }
    format!("{}: Tectonic compilation failed", input.display())
}
fn forward(
    file: &Path,
    text: &str,
    range: std::ops::Range<usize>,
    pdf: &Path,
    cancelled: &dyn Fn() -> bool,
) -> Option<SourcePoint> {
    // Preamble and deletion-only edits do not identify rendered content reliably.
    if range.is_empty()
        || (text.contains("\\begin{document}") && range.start < text.find("\\begin{document}")?)
    {
        return None;
    }
    let prefix = &text[..range.start];
    let line = prefix.bytes().filter(|c| *c == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next()?.chars().count() + 1;
    let mut out = crate::process::run(
        Command::new("synctex")
            .arg("view")
            .arg("-i")
            .arg(format!("{line}:{column}:{}", file.display()))
            .arg("-o")
            .arg(pdf),
        65536,
        cancelled,
    )
    .ok()?;
    use std::io::{Read, Seek};
    out.rewind().ok()?;
    let mut result = String::new();
    out.read_to_string(&mut result).ok()?;
    let (page, x, y) = parse_synctex(&result)?;
    Some(SourcePoint {
        file: file.to_owned(),
        start: range.start,
        end: range.end,
        page: page.checked_sub(1)?,
        x,
        y,
    })
}
fn parse_synctex(text: &str) -> Option<(u32, f64, f64)> {
    let mut found = Vec::new();
    let mut page = None;
    let mut x = None;
    let mut y = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("Page:") {
            if let (Some(p), Some(x), Some(y)) = (page.take(), x.take(), y.take()) {
                found.push((p, x, y));
            }
            page = v.trim().parse().ok();
        }
        if let Some(v) = line.strip_prefix("x:") {
            x = v.trim().parse::<f64>().ok().filter(|v| v.is_finite());
        }
        if let Some(v) = line.strip_prefix("y:") {
            y = v.trim().parse::<f64>().ok().filter(|v| v.is_finite());
        }
    }
    if let (Some(p), Some(x), Some(y)) = (page, x, y) {
        found.push((p, x, y));
    }
    found.dedup();
    if found.len() == 1 {
        found.pop()
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mappings_reject_ambiguity() {
        assert_eq!(parse_synctex("Page:2\nx:12\ny:30\n"), Some((2, 12., 30.)));
        assert!(parse_synctex("Page:1\nx:12\ny:30\nPage:2\nx:12\ny:30").is_none());
        assert!(parse_synctex("Page:1\nx:NaN\ny:30").is_none());
    }
    #[test]
    fn tectonic_unescaped_paths_and_outdir_rebasing() {
        let root = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("chapter files")).unwrap();
        let path = root.path().join("chapter files/part one.tex");
        std::fs::write(&path, "text").unwrap();
        let rules = format!(
            "document.pdf : {} \\\n  {}\n",
            path.display(),
            out.path().join("chapter files/part one.tex").display()
        );
        assert_eq!(
            make_dependencies(&rules, root.path(), out.path()),
            BTreeSet::from([path])
        );
    }
    #[test]
    fn escaped_dependencies() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a b.tex"), "").unwrap();
        assert_eq!(
            make_dependencies(
                "out.pdf: a\\ b.tex \\\n",
                d.path(),
                Path::new("/not-output")
            )
            .len(),
            1
        );
    }
}
