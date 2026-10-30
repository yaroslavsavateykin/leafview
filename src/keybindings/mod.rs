//! Unified keyboard resolution, command metadata and settings UI.
pub mod config;
pub mod engine;
pub mod keys;
pub mod registry;
#[cfg(test)]
mod tests;
mod ui;
pub use engine::Resolution;
pub use registry::Invocation;
use std::{path::Path, time::Instant};
pub struct System {
    pub registry: registry::Registry,
    pub engine: engine::Engine,
    pub config: config::Config,
    pub ui: Option<ui::Panel>,
    pub error: String,
    paths: Vec<std::path::PathBuf>,
    stamps: Vec<Option<std::time::SystemTime>>,
    last_check: Instant,
}
impl System {
    pub fn new(root: &Path) -> Self {
        let registry = registry::Registry::builtins();
        let paths = config::paths(root);
        let loaded = config::load(&paths, &registry);
        let error = loaded.as_ref().err().cloned().unwrap_or_default();
        let (config, bindings) = loaded.unwrap_or_else(|_| config::load(&[], &registry).unwrap());
        Self {
            engine: engine::Engine::compile(bindings, config.timeout_ms),
            registry,
            config,
            ui: None,
            error,
            stamps: vec![],
            paths,
            last_check: Instant::now(),
        }
    }
    /// Validate command identity/availability before any frontend executes it.
    pub fn validate(&self, invocation: &Invocation) -> Result<(), String> {
        match self.registry.commands.get(&invocation.command) {
            None => Err(format!("Unknown command: {}", invocation.command)),
            Some(c) if !c.available => Err(format!("{} unavailable in this build", c.description)),
            Some(_) => Ok(()),
        }
    }
    /// Apply temporary extension bindings at the highest layer. Command IDs must
    /// already be registered; the caller owns its command execution handler.
    #[allow(dead_code)] // Extension API; no dynamic plugin loader is shipped yet.
    pub fn override_temporarily(
        &mut self,
        mut bindings: Vec<engine::Binding>,
    ) -> Result<(), String> {
        for binding in &mut bindings {
            if let Some(invocation) = &binding.invocation {
                self.validate(invocation)?;
            }
            binding.priority = 10000;
        }
        let mut all = self.engine.bindings.clone();
        all.extend(bindings);
        self.engine = engine::Engine::compile(all, self.config.timeout_ms);
        Ok(())
    }
    pub fn reload(&mut self) -> Result<(), String> {
        let (config, bindings) = config::load(&self.paths, &self.registry)?;
        self.engine = engine::Engine::compile(bindings, config.timeout_ms);
        self.config = config;
        self.error.clear();
        Ok(())
    }
    pub fn poll_reload(&mut self) -> bool {
        if self.last_check.elapsed() < std::time::Duration::from_millis(750) {
            return false;
        }
        self.last_check = Instant::now();
        let stamps = self
            .paths
            .iter()
            .map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
            .collect::<Vec<_>>();
        if stamps == self.stamps {
            return false;
        }
        self.stamps = stamps;
        if let Err(e) = self.reload() {
            self.error = e;
        }
        true
    }
    pub fn open(&mut self, settings: bool) {
        self.engine.reset();
        self.ui = Some(ui::Panel::new(settings));
    }
    pub fn render(&self, f: &mut ratatui::Frame) {
        ui::render(self, f);
    }
    pub fn panel_key(&mut self, key: crossterm::event::KeyEvent) -> Option<Invocation> {
        ui::key(self, key)
    }
    pub fn active_panel(&self) -> bool {
        self.ui.is_some()
    }
}
