//! Typst 0.15 compile options supported by the paged live viewer.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, path::PathBuf};
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub root: Option<PathBuf>,
    pub inputs: Vec<(String, String)>,
    pub fonts: Vec<PathBuf>,
    pub ignore_system: bool,
    pub ignore_embedded: bool,
    pub package: Option<PathBuf>,
    pub cache: Option<PathBuf>,
    pub cert: Option<PathBuf>,
    pub timestamp: Option<i64>,
    pub jobs: Option<usize>,
    pub features: Vec<String>,
    pub pretty: bool,
    pub no_tags: bool,
    pub no_pdf: bool,
    pub standards: Vec<String>,
    pub pages: Option<String>,
}
impl Options {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<(PathBuf, Self)> {
        let mut options = Self {
            root: std::env::var_os("TYPST_ROOT").map(Into::into),
            fonts: std::env::var_os("TYPST_FONT_PATHS")
                .map(|p| std::env::split_paths(&p).collect())
                .unwrap_or_default(),
            ignore_system: env_bool("TYPST_IGNORE_SYSTEM_FONTS")?,
            ignore_embedded: env_bool("TYPST_IGNORE_EMBEDDED_FONTS")?,
            package: std::env::var_os("TYPST_PACKAGE_PATH").map(Into::into),
            cache: std::env::var_os("TYPST_PACKAGE_CACHE_PATH").map(Into::into),
            cert: std::env::var_os("TYPST_CERT").map(Into::into),
            timestamp: std::env::var("SOURCE_DATE_EPOCH")
                .ok()
                .map(|v| v.parse())
                .transpose()?,
            features: std::env::var("TYPST_FEATURES")
                .ok()
                .map(|v| v.split(',').map(str::to_owned).collect())
                .unwrap_or_default(),
            ..Self::default()
        };
        let mut input = None;
        let mut args = args.into_iter();
        let mut literal = false;
        while let Some(arg) = args.next() {
            if arg == "--" && !literal {
                literal = true;
                continue;
            }
            let text = arg.to_string_lossy();
            if literal || !text.starts_with('-') {
                anyhow::ensure!(
                    input.is_none(),
                    "only one Typst input is allowed; output is managed privately by LeafView"
                );
                input = Some(PathBuf::from(arg));
                continue;
            }
            let (flag, inline) = text
                .split_once('=')
                .map_or((text.as_ref(), None), |(a, b)| (a, Some(OsString::from(b))));
            let mut value = || -> Result<OsString> {
                inline
                    .clone()
                    .or_else(|| args.next())
                    .with_context(|| format!("{flag} requires a value"))
            };
            match flag {
                "--root" => options.root = Some(value()?.into()),
                "--font-path" => options.fonts.extend(std::env::split_paths(&value()?)),
                "--package-path" => options.package = Some(value()?.into()),
                "--package-cache-path" => options.cache = Some(value()?.into()),
                "--cert" => options.cert = Some(value()?.into()),
                "--input" => { let v = value()?.into_string().map_err(|_|anyhow::anyhow!("--input must be UTF-8"))?; let (k,v) = v.split_once('=').context("--input requires key=value")?; anyhow::ensure!(!k.trim().is_empty(), "empty --input key"); options.inputs.push((k.trim().to_owned(),v.trim().to_owned())); }
                "--creation-timestamp" => options.timestamp = Some(value()?.to_string_lossy().parse()?),
                "--jobs" | "-j" => { let n = value()?.to_string_lossy().parse()?; anyhow::ensure!(n>0,"jobs must be positive"); options.jobs = Some(n); }
                "--features" => options.features = value()?.to_string_lossy().split(',').map(str::to_owned).collect(),
                "--pdf-standard" => options.standards.extend(value()?.to_string_lossy().split(',').map(str::to_owned)),
                "--pages" => options.pages = Some(value()?.to_string_lossy().into_owned()),
                "--no-pdf" => { anyhow::ensure!(inline.is_none(), "--no-pdf takes no value"); options.no_pdf = true; }
                "--pretty" | "--no-pdf-tags" | "--ignore-system-fonts" | "--ignore-embedded-fonts" => {
                    anyhow::ensure!(inline.is_none(), "{flag} takes no value");
                    match flag { "--pretty" => options.pretty=true, "--no-pdf-tags" => options.no_tags=true, "--ignore-system-fonts" => options.ignore_system=true, _ => options.ignore_embedded=true }
                }
                "--format" | "-f" => anyhow::ensure!(value()? == "pdf", "LeafView live preview requires --format pdf (PNG/SVG/HTML/bundle cannot provide PDF synchronization)"),
                "--diagnostic-format" => { let v=value()?; anyhow::ensure!(v=="human" || v=="short", "invalid diagnostic format"); }
                "--color" => { let v=value()?; anyhow::ensure!(v=="auto" || v=="always" || v=="never", "invalid color mode"); }
                "--ppi" => { let n:f64=value()?.to_string_lossy().parse()?; anyhow::ensure!(n.is_finite() && n>0.,"invalid PPI"); /* PNG export only, no PDF effect */ }
                "--deps" | "--deps-format" | "--make-deps" | "--timings" | "--open" | "--no-serve" | "--no-reload" | "--port" => bail!("{flag} is not supported in LeafView live preview: output/lifecycle is owned by the viewer; use typst compile/watch for CLI export side effects"),
                _ if text.starts_with("-j") && !text.starts_with("--") => { let n=text[2..].parse()?; anyhow::ensure!(n>0,"jobs must be positive"); options.jobs=Some(n); }
                _ => bail!("unknown Typst option: {flag}; run leafview typst --help"),
            }
        }
        Ok((
            input.context("missing Typst input; use leafview typst <file.typ> [OPTIONS]")?,
            options,
        ))
    }
}
fn env_bool(key: &str) -> Result<bool> {
    match std::env::var(key).as_deref() {
        Ok("true" | "1") => Ok(true),
        Ok("false" | "0") | Err(_) => Ok(false),
        _ => bail!("invalid boolean in {key}"),
    }
}
pub const HELP: &str = "Usage: leafview typst [OPTIONS] <file.typ>\n\nCompilation: --root DIR, --input key=value (repeatable), --font-path DIR,\n  --ignore-system-fonts, --ignore-embedded-fonts, --package-path DIR,\n  --package-cache-path DIR, --cert PATH, --creation-timestamp SECONDS,\n  -j/--jobs N, --features LIST, --pdf-standard LIST, --pages RANGES,\n  --no-pdf-tags, --pretty, -f/--format pdf.\nUI diagnostics: --diagnostic-format human|short, --color auto|always|never\n  (accepted; LeafView always sanitizes short status messages).\n--ppi is accepted but affects PNG export only, not PDF.\nExport/server flags --deps, --deps-format, --make-deps, --timings, --open,\n  --no-serve, --no-reload, --port are rejected explicitly.\nSuccessful builds atomically update <input>.pdf beside the input.\nUse --no-pdf to keep output private (existing PDF is left untouched).\n--help/-h shows this help; --version/-V shows the embedded compiler version.";
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn root_inputs_and_order() {
        let (p, o) = Options::parse(
            [
                "--root=../..",
                "file with spaces.typ",
                "--input",
                "x=a=b",
                "-j2",
                "--pages",
                "2-5",
            ]
            .map(Into::into),
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("file with spaces.typ"));
        assert_eq!(o.root, Some("../..".into()));
        assert_eq!(o.inputs[0], ("x".into(), "a=b".into()));
        assert_eq!(o.jobs, Some(2));
        assert!(Options::parse(["x.typ", "--format", "html"].map(Into::into)).is_err());
        assert!(Options::parse(["x.typ", "--wat"].map(Into::into)).is_err());
    }
}
