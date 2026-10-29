//! Flat, editable compiler flags with per-document overrides in one shared JSON.
use crate::typst_options::{Options, Patch};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Flags {
    #[serde(flatten)]
    pub options: Patch,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_pdf: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compile: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_sync: Option<bool>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Config {
    #[serde(flatten)]
    pub flags: Flags,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub files: BTreeMap<PathBuf, Flags>,
}
impl<'de> Deserialize<'de> for Config {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        use serde::de::Error;
        let mut value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| D::Error::custom("config must be an object"))?;
        let files = object
            .remove("files")
            .map(serde_json::from_value)
            .transpose()
            .map_err(D::Error::custom)?
            .unwrap_or_default();
        let flags = serde_json::from_value(value).map_err(D::Error::custom)?;
        Ok(Self { flags, files })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Effective {
    pub options: Options,
    pub auto_compile: bool,
    pub auto_sync: bool,
}
impl Default for Effective {
    fn default() -> Self {
        Self {
            options: Options::default(),
            auto_compile: true,
            auto_sync: true,
        }
    }
}
impl Flags {
    pub fn apply(&self, value: &mut Effective) {
        // An explicit inputs object replaces the inherited map, so deleting keys works.
        if self.options.inputs.is_some() {
            value.options.inputs.clear();
        }
        self.options.apply(&mut value.options);
        if let Some(write) = self.write_pdf {
            value.options.no_pdf = !write;
        }
        if let Some(auto) = self.auto_compile {
            value.auto_compile = auto;
        }
        if let Some(sync) = self.auto_sync {
            value.auto_sync = sync;
        }
    }
    pub fn snapshot(value: &Effective) -> Result<Self> {
        let mut json = serde_json::to_value(&value.options)?;
        json["inputs"] = serde_json::to_value(
            value
                .options
                .inputs
                .iter()
                .cloned()
                .collect::<BTreeMap<_, _>>(),
        )?;
        json.as_object_mut().unwrap().remove("no_pdf");
        let options = serde_json::from_value(json)?;
        Ok(Self {
            options,
            write_pdf: Some(!value.options.no_pdf),
            auto_compile: Some(value.auto_compile),
            auto_sync: Some(value.auto_sync),
        })
    }
    pub fn resolve_paths(&mut self, base: &Path) {
        let absolute = |p: &mut PathBuf| {
            if p.is_relative() {
                *p = base.join(&*p);
            }
        };
        for p in [
            &mut self.options.root,
            &mut self.options.package,
            &mut self.options.cache,
            &mut self.options.cert,
        ]
        .into_iter()
        .flatten()
        .flatten()
        {
            absolute(p);
        }
        if let Some(fonts) = &mut self.options.fonts {
            for p in fonts {
                absolute(p);
            }
        }
    }
}
impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        anyhow::ensure!(
            std::fs::metadata(path)?.len() <= 1024 * 1024,
            "config exceeds 1 MiB"
        );
        let config: Self = serde_json::from_reader(std::fs::File::open(path)?)
            .with_context(|| format!("invalid config {}", path.display()))?;
        config.check()?;
        Ok(config)
    }
    fn check(&self) -> Result<()> {
        anyhow::ensure!(self.files.len() <= 4096, "too many file entries");
        for flags in std::iter::once(&self.flags).chain(self.files.values()) {
            let mut value = Effective::default();
            flags.apply(&mut value);
            validate_values(&value.options)?;
        }
        anyhow::ensure!(
            self.files.keys().all(|p| !p.as_os_str().is_empty()),
            "empty file key"
        );
        Ok(())
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        self.check()?;
        anyhow::ensure!(
            serde_json::to_vec_pretty(self)?.len() <= 1024 * 1024,
            "config exceeds 1 MiB"
        );
        let parent = path.parent().context("config has no parent")?;
        std::fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut temp, self)?;
        temp.write_all(
            b"
",
        )?;
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
}
#[derive(Clone)]
pub struct Session {
    pub global: Config,
    pub project: Config,
    pub runtime: Option<Flags>,
    pub global_path: PathBuf,
    pub project_path: PathBuf,
    pub input: PathBuf,
    pub cli: Flags,
    cli_base: PathBuf,
    pub warning: Option<String>,
}
impl Session {
    pub fn load(input: &Path, cli: Flags) -> Self {
        let global_path = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
            .unwrap_or_else(|| input.parent().unwrap().join(".config"))
            .join("leafview/config.json");
        let global_path = if global_path.is_relative() {
            std::env::current_dir()
                .unwrap_or_else(|_| input.parent().unwrap().to_owned())
                .join(global_path)
        } else {
            global_path
        };
        let project_path = input
            .parent()
            .unwrap()
            .ancestors()
            .map(|p| p.join(".leafview.json"))
            .find(|p| p.is_file())
            .unwrap_or_else(|| input.parent().unwrap().join(".leafview.json"));
        Self::from_paths(input.to_owned(), global_path, project_path, cli)
    }
    pub fn from_paths(
        input: PathBuf,
        global_path: PathBuf,
        project_path: PathBuf,
        cli: Flags,
    ) -> Self {
        let mut warnings = Vec::new();
        let mut load = |p: &Path| {
            Config::load(p).unwrap_or_else(|e| {
                warnings.push(format!("{e:#}"));
                Config::default()
            })
        };
        let global = load(&global_path);
        let project = load(&project_path);
        Self {
            input: input.canonicalize().unwrap_or(input),
            global,
            project,
            runtime: None,
            global_path,
            project_path,
            cli,
            cli_base: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            warning: if warnings.is_empty() {
                None
            } else {
                Some(warnings.join(
                    "
",
                ))
            },
        }
    }
    fn file_entry<'a>(&self, config: &'a Config, path: &Path) -> Option<&'a Flags> {
        config.files.get(&self.input).or_else(|| {
            config.files.iter().find_map(|(key, value)| {
                let key = if key.is_absolute() {
                    key.clone()
                } else {
                    path.parent().unwrap().join(key)
                };
                (key.canonicalize().ok().as_ref() == Some(&self.input)).then_some(value)
            })
        })
    }
    pub fn effective(&self) -> Result<Effective> {
        self.merge(true)
    }
    pub fn editable(&self) -> Result<Effective> {
        self.merge(false)
    }
    fn merge(&self, resolve: bool) -> Result<Effective> {
        let mut value = Effective::default();
        for (config, path) in [
            (&self.global, &self.global_path),
            (&self.project, &self.project_path),
        ] {
            let mut flags = config.flags.clone();
            if resolve {
                flags.resolve_paths(path.parent().unwrap());
            }
            flags.apply(&mut value);
        }
        for (config, path) in [
            (&self.global, &self.global_path),
            (&self.project, &self.project_path),
        ] {
            if let Some(flags) = self.file_entry(config, path) {
                let mut flags = flags.clone();
                if resolve {
                    flags.resolve_paths(self.input.parent().unwrap());
                }
                flags.apply(&mut value);
            }
        }
        let mut cli = self.cli.clone();
        if resolve {
            cli.resolve_paths(&self.cli_base);
        }
        cli.apply(&mut value);
        if let Some(flags) = &self.runtime {
            let mut flags = flags.clone();
            if resolve {
                flags.resolve_paths(self.input.parent().unwrap());
            }
            flags.apply(&mut value);
        }
        validate_values(&value.options)?;
        Ok(value)
    }
    pub fn initial(&mut self, input: &Path) -> Effective {
        match self.effective().and_then(|v| {
            validate(input, &v.options)?;
            Ok(v)
        }) {
            Ok(v) => v,
            Err(e) => {
                self.warning = Some(format!(
                    "{}
Configuration not applied: {e:#}",
                    self.warning.take().unwrap_or_default()
                ));
                Effective::default()
            }
        }
    }
    pub fn save(&self) -> Result<()> {
        // Always re-read to preserve settings for all other documents.
        let mut config = Config::load(&self.global_path)?;
        let flags = self.runtime.clone().context("no edited settings to save")?;
        config.files.insert(self.input.clone(), flags);
        config.save(&self.global_path)
    }
}
pub fn validate_values(o: &Options) -> Result<()> {
    anyhow::ensure!(
        o.jobs.is_none_or(|n| n > 0 && n <= 256),
        "jobs must be between 1 and 256"
    );
    anyhow::ensure!(
        o.inputs.len() <= 256
            && o.inputs
                .iter()
                .all(|(k, v)| !k.trim().is_empty() && k.len() <= 1024 && v.len() <= 32768),
        "invalid or oversized inputs"
    );
    for name in &o.features {
        anyhow::ensure!(
            ["html", "bundle", "a11y-extras"].contains(&name.as_str()),
            "unknown Typst feature: {name}"
        );
    }
    let standards = o
        .standards
        .iter()
        .map(|s| serde_json::from_value(serde_json::Value::String(s.clone())))
        .collect::<std::result::Result<Vec<typst_pdf::PdfStandard>, _>>()?;
    typst_pdf::PdfStandards::new(&standards).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    if let Some(pages) = &o.pages {
        crate::typst_compiler::parse_pages(pages)?;
    }
    if let Some(timestamp) = o.timestamp {
        typst_kit::datetime::Time::fixed_timestamp(timestamp)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    Ok(())
}
pub fn validate(input: &Path, o: &Options) -> Result<PathBuf> {
    validate_values(o)?;
    anyhow::ensure!(
        serde_json::to_vec(o)?.len() <= 48 * 1024,
        "compiler options exceed 48 KiB"
    );
    let root = o
        .root
        .as_deref()
        .unwrap_or_else(|| input.parent().unwrap())
        .canonicalize()
        .context("invalid project root")?;
    anyhow::ensure!(
        root.is_dir() && input.starts_with(&root),
        "input must be within project root {}",
        root.display()
    );
    for p in &o.fonts {
        anyhow::ensure!(p.is_dir(), "font path is not a directory: {}", p.display());
    }
    for p in [&o.package, &o.cache].into_iter().flatten() {
        anyhow::ensure!(
            !p.exists() || p.is_dir(),
            "package path is not a directory: {}",
            p.display()
        );
    }
    if let Some(p) = &o.cert {
        anyhow::ensure!(p.is_file(), "certificate does not exist: {}", p.display());
    }
    Ok(root)
}
// The existing parser remains the single CLI grammar; explicit fields form the CLI layer.
pub fn parse_cli(args: Vec<OsString>) -> Result<(PathBuf, Flags)> {
    let filtered = args;
    let (path, options) = Options::parse(filtered.clone())?;
    let no_pdf = options.no_pdf;
    let mut all = serde_json::to_value(Flags::snapshot(&Effective {
        options,
        ..Effective::default()
    })?)?;
    all["no_pdf"] = serde_json::Value::Bool(no_pdf);
    let mut patch = serde_json::Map::new();
    let mapping = [
        ("--root", "root"),
        ("--input", "inputs"),
        ("--font-path", "fonts"),
        ("--ignore-system-fonts", "ignore_system"),
        ("--ignore-embedded-fonts", "ignore_embedded"),
        ("--package-path", "package"),
        ("--package-cache-path", "cache"),
        ("--cert", "cert"),
        ("--creation-timestamp", "timestamp"),
        ("--jobs", "jobs"),
        ("-j", "jobs"),
        ("--features", "features"),
        ("--pretty", "pretty"),
        ("--no-pdf-tags", "no_tags"),
        ("--output", "output"),
        ("--write-pdf", "no_pdf"),
        ("--pdf-standard", "standards"),
        ("--pages", "pages"),
    ];
    // Environment is treated as an explicit CLI-level fallback, as before.
    for (env, field) in [
        ("TYPST_ROOT", "root"),
        ("TYPST_FONT_PATHS", "fonts"),
        ("TYPST_IGNORE_SYSTEM_FONTS", "ignore_system"),
        ("TYPST_IGNORE_EMBEDDED_FONTS", "ignore_embedded"),
        ("TYPST_PACKAGE_PATH", "package"),
        ("TYPST_PACKAGE_CACHE_PATH", "cache"),
        ("TYPST_CERT", "cert"),
        ("SOURCE_DATE_EPOCH", "timestamp"),
        ("TYPST_FEATURES", "features"),
    ] {
        if std::env::var_os(env).is_some() {
            patch.insert(field.into(), all[field].clone());
        }
    }
    let mut literal = false;
    let mut skip_value = false;
    for arg in &filtered {
        if skip_value {
            skip_value = false;
            continue;
        }
        if arg == "--" {
            literal = true;
        }
        if literal {
            continue;
        }
        let text = arg.to_string_lossy();
        for (flag, field) in mapping {
            if text == flag
                || text.starts_with(&format!("{flag}="))
                || (flag == "-j" && text.starts_with("-j") && !text.starts_with("--"))
            {
                patch.insert(field.into(), all[field].clone());
            }
        }
        skip_value = text.starts_with('-')
            && !text.contains('=')
            && ![
                "--ignore-system-fonts",
                "--ignore-embedded-fonts",
                "--pretty",
                "--no-pdf-tags",
                "--write-pdf",
            ]
            .contains(&text.as_ref())
            && !(text.starts_with("-j") && text.len() > 2 && !text.starts_with("--"));
    }
    let flags: Flags = serde_json::from_value(patch.into())?;
    Ok((path, flags))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flat_file_flags_round_trip_and_preserve_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.typ");
        let b = dir.path().join("b.typ");
        std::fs::write(&a, "").unwrap();
        std::fs::write(&b, "").unwrap();
        let global = dir.path().join("config.json");
        let project = dir.path().join(".leafview.json");
        std::fs::write(
            &global,
            serde_json::json!({"jobs":1,"files":{b.to_str().unwrap():{"jobs":4}}}).to_string(),
        )
        .unwrap();
        let mut s =
            Session::from_paths(a.clone(), global.clone(), project.clone(), Flags::default());
        assert_eq!(s.effective().unwrap().options.jobs, Some(1));
        let mut value = s.effective().unwrap();
        value.options.jobs = Some(2);
        value.options.inputs = vec![("x".into(), "yes".into())];
        s.runtime = Some(Flags::snapshot(&value).unwrap());
        s.save().unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&global).unwrap()).unwrap();
        assert!(json.get("version").is_none() && json.get("profiles").is_none());
        assert_eq!(json["files"][b.to_str().unwrap()]["jobs"], 4);
        let loaded = Session::from_paths(a, global.clone(), project, Flags::default());
        assert_eq!(loaded.effective().unwrap().options.jobs, Some(2));
        assert_eq!(
            loaded.effective().unwrap().options.inputs,
            value.options.inputs
        );
        std::fs::write(&global, "{invalid").unwrap();
        assert!(s.save().is_err());
        assert_eq!(std::fs::read_to_string(global).unwrap(), "{invalid");
    }
    #[test]
    fn pdf_export_default_and_cli_override() {
        let (_, flags) = parse_cli(vec!["main.typ".into()]).unwrap();
        let mut value = Effective::default();
        flags.apply(&mut value);
        assert!(value.options.no_pdf);
        let (_, flags) = parse_cli(vec!["main.typ".into(), "--write-pdf".into()]).unwrap();
        flags.apply(&mut value);
        assert!(!value.options.no_pdf);
        let flags = Flags {
            write_pdf: Some(false),
            ..Default::default()
        };
        flags.apply(&mut value);
        assert!(value.options.no_pdf);
        let (_, flags) = parse_cli(vec![
            "main.typ".into(),
            "--write-pdf".into(),
            "--output".into(),
            "result.pdf".into(),
        ])
        .unwrap();
        flags.apply(&mut value);
        assert!(!value.options.no_pdf);
        assert_eq!(value.options.output, Some("result.pdf".into()));
    }
    #[test]
    fn paths_remain_literal_in_editor_and_saved_file() {
        let d = tempfile::tempdir().unwrap();
        let input = d.path().join("a.typ");
        std::fs::write(&input, "").unwrap();
        let global = d.path().join("config.json");
        let mut session = Session::from_paths(
            input.clone(),
            global.clone(),
            d.path().join(".leafview.json"),
            Flags::default(),
        );
        let mut raw = Effective::default();
        raw.options.root = Some("../..".into());
        session.runtime = Some(Flags::snapshot(&raw).unwrap());
        session.save().unwrap();
        let loaded = Session::from_paths(
            input,
            global,
            d.path().join(".leafview.json"),
            Flags::default(),
        );
        assert_eq!(
            loaded.editable().unwrap().options.root,
            Some("../..".into())
        );
        assert_eq!(
            loaded.effective().unwrap().options.root,
            Some(d.path().join("../.."))
        );
    }
    #[test]
    fn rejects_metadata_and_bad_values_and_clears_options() {
        assert!(serde_json::from_str::<Config>(r#"{"profiles":{}}"#).is_err());
        let flags: Flags = serde_json::from_str(r#"{"jobs":0}"#).unwrap();
        let mut value = Effective::default();
        flags.apply(&mut value);
        assert!(validate_values(&value.options).is_err());
        value.options.jobs = Some(4);
        value.options.inputs = vec![("old".into(), "value".into())];
        let flags: Flags = serde_json::from_str(r#"{"jobs":null,"inputs":{}}"#).unwrap();
        flags.apply(&mut value);
        assert!(value.options.jobs.is_none() && value.options.inputs.is_empty());
        assert!(parse_cli(["a.typ", "--profile", "teacher"].map(Into::into).into()).is_err());
        let (_, flags) =
            parse_cli(["a.typ", "-j2", "--root", "../.."].map(Into::into).into()).unwrap();
        assert_eq!(flags.options.jobs, Some(Some(2)));
        assert_eq!(flags.options.root, Some(Some("../..".into())));
    }
}
