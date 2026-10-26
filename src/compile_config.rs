//! Versioned configuration layers. Compiler fields come from typst_options::Patch.
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
pub struct Profile {
    #[serde(flatten)]
    pub options: Patch,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_pdf: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compile: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_sync: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_inputs: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub default_profile: String,
    pub profiles: BTreeMap<String, Profile>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            default_profile: "default".into(),
            profiles: BTreeMap::new(),
        }
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
impl Profile {
    fn apply(&self, target: &mut Effective) {
        if self.replace_inputs == Some(true) {
            target.options.inputs.clear();
        }
        self.options.apply(&mut target.options);
        if let Some(write) = self.write_pdf {
            target.options.no_pdf = !write;
        }
        if let Some(auto) = self.auto_compile {
            target.auto_compile = auto;
        }
        if let Some(sync) = self.auto_sync {
            target.auto_sync = sync;
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
        let options = serde_json::from_value(json)?;
        Ok(Self {
            options,
            write_pdf: Some(!value.options.no_pdf),
            auto_compile: Some(value.auto_compile),
            auto_sync: Some(value.auto_sync),
            replace_inputs: Some(true),
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
    fn check(&self) -> Result<()> {
        anyhow::ensure!(
            self.version == 1,
            "unsupported LeafView config version {}",
            self.version
        );
        anyhow::ensure!(
            !self.default_profile.trim().is_empty(),
            "default_profile is empty"
        );
        anyhow::ensure!(
            self.profiles.len() <= 128,
            "too many profiles (maximum 128)"
        );
        for (name, profile) in &self.profiles {
            anyhow::ensure!(
                !name.trim().is_empty() && name.len() <= 128,
                "invalid profile name"
            );
            let mut value = Effective::default();
            profile.apply(&mut value);
            validate_values(&value.options)?;
        }
        Ok(())
    }
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
        temp.write_all(b"\n")?;
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
}
#[derive(Clone)]
pub struct Session {
    pub global: Config,
    pub project: Config,
    pub runtime: Config,
    pub selected: String,
    pub global_path: PathBuf,
    pub project_path: PathBuf,
    pub cli: Profile,
    pub warning: Option<String>,
}
impl Session {
    pub fn load(input: &Path, cli: Profile, selected: Option<String>) -> Self {
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
        Self::from_paths(global_path, project_path, cli, selected)
    }
    pub fn from_paths(
        global_path: PathBuf,
        project_path: PathBuf,
        cli: Profile,
        selected: Option<String>,
    ) -> Self {
        let mut warnings = Vec::new();
        let mut load = |path: &Path| {
            Config::load(path).unwrap_or_else(|e| {
                warnings.push(format!("{e:#}"));
                Config::default()
            })
        };
        let global = load(&global_path);
        let project = load(&project_path);
        let chosen = selected.unwrap_or_else(|| {
            if project_path.exists() {
                project.default_profile.clone()
            } else {
                global.default_profile.clone()
            }
        });
        Self {
            global,
            project,
            runtime: Config::default(),
            selected: chosen,
            global_path,
            project_path,
            cli,
            warning: if warnings.is_empty() {
                None
            } else {
                Some(warnings.join("\n"))
            },
        }
    }
    pub fn initial(&mut self, input: &Path) -> Effective {
        match self.effective().and_then(|value| {
            validate(input, &value.options)?;
            Ok(value)
        }) {
            Ok(value) => value,
            Err(error) => {
                let previous = self.warning.take().unwrap_or_default();
                self.warning = Some(format!("{previous}\nConfiguration not applied: {error:#}"));
                Effective::default()
            }
        }
    }
    pub fn names(&self) -> Vec<String> {
        let mut names: std::collections::BTreeSet<String> =
            ["default", "teacher", "student"].map(str::to_owned).into();
        for config in [&self.global, &self.project, &self.runtime] {
            names.extend(config.profiles.keys().cloned());
        }
        names.insert(self.selected.clone());
        names.into_iter().collect()
    }
    pub fn effective(&self) -> Result<Effective> {
        anyhow::ensure!(
            !self.selected.trim().is_empty() && self.selected.len() <= 128,
            "invalid profile name"
        );
        let mut value = Effective::default();
        for (config, base) in [
            (&self.global, self.global_path.parent().unwrap()),
            (&self.project, self.project_path.parent().unwrap()),
        ] {
            if let Some(profile) = config.profiles.get(&self.selected) {
                let mut profile = profile.clone();
                profile.resolve_paths(base);
                profile.apply(&mut value);
            }
        }
        self.cli.apply(&mut value);
        if let Some(profile) = self.runtime.profiles.get(&self.selected) {
            profile.apply(&mut value);
        }
        validate_values(&value.options)?;
        anyhow::ensure!(
            serde_json::to_vec(&value.options)?.len() <= 48 * 1024,
            "compiler options exceed 48 KiB"
        );
        Ok(value)
    }
    pub fn save(&self, global: bool) -> Result<()> {
        let (mut config, path) = if global {
            (self.global.clone(), &self.global_path)
        } else {
            (self.project.clone(), &self.project_path)
        };
        // Persist runtime snapshots only; other profiles and their relative paths stay intact.
        // Resolve from disk again so an editor's unrelated changes are not lost.
        if path.exists() {
            config = Config::load(path)?;
        }
        config.profiles.extend(self.runtime.profiles.clone());
        config.default_profile = self.selected.clone();
        config.save(path)
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
pub fn parse_cli(args: Vec<OsString>) -> Result<(PathBuf, Profile, Option<String>)> {
    let mut filtered = Vec::new();
    let mut selected = None;
    let mut iter = args.into_iter();
    let mut literal = false;
    while let Some(arg) = iter.next() {
        if arg == "--" {
            literal = true;
        }
        if !literal && arg == "--profile" {
            selected = Some(
                iter.next()
                    .context("--profile requires a name")?
                    .into_string()
                    .map_err(|_| anyhow::anyhow!("profile must be UTF-8"))?,
            );
        } else if !literal && arg.to_string_lossy().starts_with("--profile=") {
            selected = Some(arg.to_string_lossy()[10..].to_owned());
        } else {
            filtered.push(arg);
        }
    }
    let (path, options) = Options::parse(filtered.clone())?;
    let all = serde_json::to_value(Profile::snapshot(&Effective {
        options,
        ..Effective::default()
    })?)?;
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
        ("--no-pdf", "no_pdf"),
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
                "--no-pdf",
            ]
            .contains(&text.as_ref())
            && !(text.starts_with("-j") && text.len() > 2 && !text.starts_with("--"));
    }
    let mut profile: Profile = serde_json::from_value(patch.into())?;
    profile.resolve_paths(&std::env::current_dir()?);
    Ok((path, profile, selected))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layers_profiles_paths_and_atomic_save() {
        let d = tempfile::tempdir().unwrap();
        let global = d.path().join("global.json");
        let project = d.path().join(".leafview.json");
        std::fs::write(&global, r#"{"version":1,"default_profile":"teacher","profiles":{"teacher":{"jobs":1,"inputs":{"mode":"global","extra":"yes"}}}}"#).unwrap();
        std::fs::write(&project, r#"{"version":1,"default_profile":"teacher","profiles":{"teacher":{"jobs":2,"root":".","inputs":{"mode":"project"}},"student":{"write_pdf":false}}}"#).unwrap();
        let cli: Profile =
            serde_json::from_value(serde_json::json!({"jobs":3,"inputs":{"mode":"cli"}})).unwrap();
        let mut s = Session::from_paths(global, project.clone(), cli, None);
        let mut v = s.effective().unwrap();
        assert_eq!(v.options.jobs, Some(3));
        assert_eq!(v.options.root, Some(d.path().join(".")));
        assert!(v.options.inputs.contains(&("extra".into(), "yes".into())));
        v.options.jobs = Some(4);
        s.runtime
            .profiles
            .insert("teacher".into(), Profile::snapshot(&v).unwrap());
        assert_eq!(s.effective().unwrap().options.jobs, Some(4));
        s.save(false).unwrap();
        assert_eq!(Config::load(&project).unwrap().default_profile, "teacher");
        s.selected = "student".into();
        assert!(s.effective().unwrap().options.no_pdf);
        assert!(d.path().read_dir().unwrap().all(|p| !p
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("tmp")));
    }
    #[test]
    fn runtime_can_clear_nullable_fields_and_inputs() {
        let d = tempfile::tempdir().unwrap();
        let global = d.path().join("global.json");
        let project = d.path().join(".leafview.json");
        std::fs::write(
            &project,
            r#"{"profiles":{"default":{"jobs":4,"pages":"1-2","inputs":{"x":"yes"}}}}"#,
        )
        .unwrap();
        let mut s = Session::from_paths(global, project, Profile::default(), None);
        let mut value = s.effective().unwrap();
        value.options.jobs = None;
        value.options.pages = None;
        value.options.inputs.clear();
        s.runtime
            .profiles
            .insert("default".into(), Profile::snapshot(&value).unwrap());
        let v = s.effective().unwrap();
        assert!(
            v.options.jobs.is_none() && v.options.pages.is_none() && v.options.inputs.is_empty()
        );
        s.save(false).unwrap();
        let reloaded = Session::from_paths(
            s.global_path.clone(),
            s.project_path.clone(),
            Profile::default(),
            None,
        );
        assert!(reloaded.effective().unwrap().options.inputs.is_empty());
    }
    #[test]
    fn invalid_json_does_not_overwrite_or_crash() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join(".leafview.json");
        std::fs::write(&p, "{bad").unwrap();
        let s = Session::from_paths(
            d.path().join("global.json"),
            p.clone(),
            Profile::default(),
            None,
        );
        assert!(s.warning.is_some());
        assert!(s.save(false).is_err());
        assert_eq!(std::fs::read_to_string(p).unwrap(), "{bad");
    }
    #[test]
    fn malformed_values_and_cli() {
        assert!(
            serde_json::from_str::<Config>(r#"{"profiles":{"x":{"jobs":0}}}"#)
                .unwrap()
                .check()
                .is_err()
        );
        assert!(serde_json::from_str::<Config>(r#"{"version":42}"#)
            .unwrap()
            .check()
            .is_err());
        assert!(serde_json::from_str::<Config>(r#"{"surprise":true}"#).is_err());
        let (p, cli, name) = parse_cli(
            [
                "main.typ",
                "--profile",
                "teacher",
                "-j2",
                "--input",
                "mode=cli",
            ]
            .map(Into::into)
            .into(),
        )
        .unwrap();
        assert_eq!(p, PathBuf::from("main.typ"));
        assert_eq!(name.as_deref(), Some("teacher"));
        assert_eq!(cli.options.jobs, Some(Some(2)));
        assert!(cli.options.fonts.is_none());
        let mut o = Options {
            pages: Some("4-2".into()),
            ..Options::default()
        };
        assert!(validate_values(&o).is_err());
        let d = tempfile::tempdir().unwrap();
        let input = d.path().join("main.typ");
        std::fs::write(&input, "").unwrap();
        o.pages = None;
        o.root = Some("/does/not/exist".into());
        assert!(validate(&input, &o).is_err());
    }
}
