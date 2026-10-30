//! Layered TOML overrides; one entry replaces one key, never the entire map.
use super::{
    engine::Binding,
    keys,
    registry::{Invocation, Registry},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Entry {
    Command(String),
    Detail {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
        args: serde_json::Value,
        #[serde(default)]
        disabled: bool,
        #[serde(default)]
        priority: i32,
    },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub leader: String,
    pub local_leader: String,
    pub timeout_ms: u64,
    pub preset: String,
    pub bindings: BTreeMap<String, BTreeMap<String, Entry>>,
    pub layout: BTreeMap<String, String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            leader: "Space".into(),
            local_leader: ",".into(),
            timeout_ms: 500,
            preset: "astronvim".into(),
            bindings: BTreeMap::new(),
            layout: BTreeMap::new(),
        }
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct File {
    keybindings: Config,
}
impl Config {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut raw: toml::Value = text.parse::<toml::Value>().map_err(|e| e.to_string())?;
        if let Some(table) = raw
            .get_mut("keybindings")
            .and_then(toml::Value::as_table_mut)
        {
            let names = table
                .keys()
                .filter(|key| {
                    ![
                        "leader",
                        "local_leader",
                        "timeout_ms",
                        "preset",
                        "bindings",
                        "layout",
                    ]
                    .contains(&key.as_str())
                })
                .cloned()
                .collect::<Vec<_>>();
            for name in names {
                if !table[&name].is_table() {
                    continue;
                }
                let value = table.remove(&name).unwrap();
                table
                    .entry("bindings")
                    .or_insert_with(|| toml::Value::Table(Default::default()))
                    .as_table_mut()
                    .ok_or("bindings must be a table")?
                    .insert(name, value);
            }
        }
        raw.try_into::<File>()
            .map(|f| f.keybindings)
            .map_err(|e| e.to_string())
    }
    pub fn export(&self) -> Result<String, String> {
        toml::to_string_pretty(&File {
            keybindings: self.clone(),
        })
        .map_err(|e| e.to_string())
    }
}
pub fn paths(root: &Path) -> Vec<PathBuf> {
    let global = std::env::var_os("LEAFVIEW_KEYBINDINGS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
                })
                .join("leafview/keybindings.toml")
        });
    let workspace = root
        .ancestors()
        .find(|p| p.join(".git").exists())
        .unwrap_or(root);
    let mut paths = vec![
        global,
        workspace.join(".leafview/keybindings.toml"),
        root.join(".leafview/project-keybindings.toml"),
    ];
    paths.dedup();
    paths
}
pub fn load(paths: &[PathBuf], registry: &Registry) -> Result<(Config, Vec<Binding>), String> {
    let mut config = Config::default();
    let mut layers = vec![];
    for path in paths {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let layer = Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
                layers.push((path.display().to_string(), layer));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    // Scalar inheritance also needs field-presence, not serde's default values.
    for (path, layer) in &layers {
        let raw: toml::Value = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())?
            .parse::<toml::Value>()
            .map_err(|e| e.to_string())?;
        let table = &raw["keybindings"];
        if table.get("leader").is_some() {
            config.leader = layer.leader.clone();
        }
        if table.get("local_leader").is_some() {
            config.local_leader = layer.local_leader.clone();
        }
        if table.get("timeout_ms").is_some() {
            config.timeout_ms = layer.timeout_ms;
        }
        if table.get("preset").is_some() {
            config.preset = layer.preset.clone();
        }
        config.layout.extend(layer.layout.clone());
    }
    if !["vim", "neovim", "astronvim", "leafview-default", "custom"]
        .contains(&config.preset.as_str())
    {
        return Err(format!("unknown preset {}", config.preset));
    }
    if config.timeout_ms == 0 || config.timeout_ms > 10000 {
        return Err("timeout_ms must be 1..10000".into());
    }
    let mut bindings = presets(&config)?;
    for (layer_index, (source, layer)) in layers.iter().enumerate() {
        for (context, entries) in &layer.bindings {
            config
                .bindings
                .entry(context.clone())
                .or_default()
                .extend(entries.clone());
            let (context, mode) = scope(context);
            for (key, entry) in entries {
                let (command, args, priority) = match entry {
                    Entry::Command(c) => (Some(c.clone()), serde_json::Value::Null, 0),
                    Entry::Detail {
                        command,
                        args,
                        disabled,
                        priority,
                    } => (
                        if *disabled { None } else { command.clone() },
                        args.clone(),
                        *priority,
                    ),
                };
                if let Some(id) = &command {
                    if !registry.commands.contains_key(id) {
                        return Err(format!("{source}: unknown command {id}"));
                    }
                }
                bindings.push(Binding {
                    context: context.into(),
                    mode: mode.into(),
                    keys: keys::sequence(key, &config.leader, &config.local_leader)?,
                    invocation: command.map(|command| Invocation {
                        command,
                        args,
                        count: 1,
                    }),
                    priority: 1000 + (layer_index as i32 * 100) + priority.clamp(-49, 49),
                    source: source.clone(),
                });
            }
        }
    }
    Ok((config, bindings))
}
pub fn scope(input: &str) -> (&str, &str) {
    match input {
        "normal" => ("Editor", "normal"),
        "insert" => ("Editor", "insert"),
        "visual" => ("Editor", "visual"),
        "visual_line" => ("Editor", "visual_line"),
        "visual_block" => ("Editor", "visual_block"),
        "operator" => ("Editor", "operator"),
        "pdf" => ("PDF", "normal"),
        "explorer" => ("Explorer", "normal"),
        "global" => ("Global", "any"),
        other => other.split_once('.').unwrap_or((other, "any")),
    }
}
fn presets(c: &Config) -> Result<Vec<Binding>, String> {
    let mut out = vec![];
    let mut add = |scope_name: &str, key: &str, id: &str| -> Result<(), String> {
        let (context, mode) = scope(scope_name);
        out.push(Binding {
            context: context.into(),
            mode: mode.into(),
            keys: keys::sequence(key, &c.leader, &c.local_leader)?,
            invocation: Some(Invocation::new(id)),
            priority: 0,
            source: format!("preset:{}", c.preset),
        });
        Ok(())
    };
    // Safety and discoverability exist even in custom mode.
    add("global", "F9", "commands.open")?;
    add("global", "F10", "keybindings.open")?;
    if c.preset == "custom" {
        return Ok(out);
    }
    for (key, id) in [
        ("Ctrl+c", "app.quit"),
        ("F2", "explorer.toggle"),
        ("F3", "files.find"),
        ("F4", "editor.toggle"),
    ] {
        add("global", key, id)?;
    }
    for (key, id) in [
        ("j", "pdf.scroll_down"),
        ("k", "pdf.scroll_up"),
        ("n", "pdf.next_page"),
        ("p", "pdf.previous_page"),
        ("gg", "pdf.first_page"),
        ("G", "pdf.last_page"),
        ("+", "pdf.zoom_in"),
        ("-", "pdf.zoom_out"),
        ("r", "pdf.reload"),
        ("0", "pdf.fit_page"),
        ("z", "pdf.fit_width"),
    ] {
        add("pdf", key, id)?;
    }
    for (key, id) in [
        ("j", "explorer.next_item"),
        ("k", "explorer.previous_item"),
        ("Enter", "explorer.open"),
    ] {
        add("explorer", key, id)?;
    }
    // Do not intercept compositional Vim operations: the existing grammar must
    // retain counts, registers, macros, dot repeat and operator pending motions.
    for (key, id) in [
        ("F1", "lsp.hover"),
        ("F12", "lsp.definition"),
        ("F7", "popup.next"),
        ("F8", "popup.close"),
        ("Ctrl+PageDown", "editor.next_buffer"),
        ("Ctrl+PageUp", "editor.previous_buffer"),
        ("Ctrl+q", "editor.close"),
    ] {
        add("Editor.any", key, id)?;
    }
    add("insert", "Ctrl+Space", "completion.show")?;
    for (key, id) in [
        ("Tab", "completion.next"),
        ("Down", "completion.next"),
        ("Ctrl+n", "completion.next"),
        ("Ctrl+j", "completion.next"),
        ("Shift+Tab", "completion.previous"),
        ("Up", "completion.previous"),
        ("Ctrl+p", "completion.previous"),
        ("Ctrl+k", "completion.previous"),
        ("Enter", "completion.accept"),
        ("Ctrl+e", "completion.hide"),
    ] {
        add("Completion.insert", key, id)?;
    }
    if c.preset == "astronvim" {
        for (key, id) in [
            ("<Leader>ff", "files.find"),
            ("<Leader>fr", "files.recent"),
            ("<Leader>e", "explorer.toggle"),
            ("<Leader>w", "editor.save"),
            ("<Leader>c", "editor.close"),
            ("<Leader>bd", "editor.close"),
            ("<Leader>bn", "editor.next_buffer"),
            ("<Leader>bp", "editor.previous_buffer"),
            ("]b", "editor.next_buffer"),
            ("[b", "editor.previous_buffer"),
            ("<Leader>lh", "lsp.signature"),
            ("gK", "lsp.signature"),
            ("Ctrl+w h", "window.left"),
            ("Ctrl+w l", "window.right"),
            ("Ctrl+w k", "window.up"),
            ("Ctrl+w j", "window.down"),
            ("Ctrl+w v", "window.vertical"),
            ("Ctrl+w s", "window.horizontal"),
            ("Ctrl+w c", "window.close"),
            ("<Leader>lf", "lsp.format"),
            ("<Leader>lR", "lsp.references"),
            ("<Leader>ld", "lsp.diagnostics"),
            ("<Leader>:a", "commands.open"),
            ("<Leader>?", "keybindings.open"),
            ("gd", "lsp.definition"),
            ("gr", "lsp.references"),
            ("K", "lsp.hover"),
        ] {
            add("normal", key, id)?;
        }
    }
    Ok(out)
}
