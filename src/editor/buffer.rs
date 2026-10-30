use anyhow::{Context, Result};
use helix_core::{
    history::{History, State},
    Rope, Selection, Transaction,
};
use std::{io::Write, path::PathBuf};

pub struct Buffer {
    pub path: PathBuf,
    pub state: State,
    history: History,
    pending: Option<(State, Transaction)>,
    pub version: u64,
    saved_revision: usize,
    pub scroll: usize,
    pub syntax: super::syntax::SyntaxCache,
    pub snippet: Option<helix_core::snippets::ActiveSnippet>,
}
impl Buffer {
    pub fn from_text(path: PathBuf, text: Rope) -> Self {
        Self {
            path,
            state: State {
                doc: text,
                selection: Selection::point(0),
            },
            history: History::default(),
            pending: None,
            version: 0,
            saved_revision: 0,
            scroll: 0,
            syntax: super::syntax::SyntaxCache::default(),
            snippet: None,
        }
    }
    pub fn cursor(&self) -> usize {
        self.state.selection.primary().head
    }
    pub fn set_cursor(&mut self, pos: usize) {
        self.state.selection = Selection::point(pos.min(self.state.doc.len_chars()));
    }
    pub fn dirty(&self) -> bool {
        self.pending.is_some() || self.history.current_revision() != self.saved_revision
    }
    pub fn line(&self) -> usize {
        self.state.doc.char_to_line(self.cursor())
    }
    pub fn start(&self, line: usize) -> usize {
        self.state
            .doc
            .line_to_char(line.min(self.state.doc.len_lines() - 1))
    }
    pub fn end(&self, line: usize) -> usize {
        let start = self.start(line);
        let line = self
            .state
            .doc
            .line(line.min(self.state.doc.len_lines() - 1));
        let mut end = start + line.len_chars();
        while end > start && matches!(self.state.doc.char(end - 1), '\n' | '\r') {
            end -= 1;
        }
        end
    }
    pub fn edit(&mut self, from: usize, to: usize, text: &str) {
        self.edit_many(vec![(
            from,
            to,
            if text.is_empty() {
                None
            } else {
                Some(text.into())
            },
        )]);
    }
    pub fn edit_many(&mut self, changes: Vec<helix_core::Change>) {
        let transaction = Transaction::change(&self.state.doc, changes.into_iter());
        if let Some(snippet) = &mut self.snippet {
            if snippet.map(transaction.changes()) {
                self.snippet = None;
            }
        }
        let original = self.state.clone(); // Rope clone shares its tree, not the file bytes.
        assert!(transaction.apply(&mut self.state.doc));
        self.pending = Some(match self.pending.take() {
            Some((before, previous)) => (before, previous.compose(transaction)),
            None => (original, transaction),
        });
        self.version += 1;
        self.set_cursor(self.cursor());
    }
    pub fn commit(&mut self) {
        if let Some((before, tx)) = self.pending.take() {
            let tx = tx.with_selection(self.state.selection.clone());
            self.history.commit_revision(&tx, &before);
        }
    }
    pub fn undo(&mut self, redo: bool) {
        self.commit();
        let tx = if redo {
            self.history.redo()
        } else {
            self.history.undo()
        }
        .cloned();
        if let Some(tx) = tx {
            tx.apply(&mut self.state.doc);
            if let Some(selection) = tx.selection() {
                self.state.selection = selection.clone();
            }
            self.version += 1;
        }
    }
    pub fn saved(&mut self, version: u64) {
        if version == self.version {
            self.commit();
            self.saved_revision = self.history.current_revision();
        }
    }
}

pub fn atomic_save(path: &std::path::Path, text: &Rope) -> Result<()> {
    let parent = path.parent().context("missing parent")?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    if let Ok(meta) = std::fs::metadata(path) {
        file.as_file().set_permissions(meta.permissions())?;
    }
    for chunk in text.chunks() {
        file.write_all(chunk.as_bytes())?;
    }
    file.as_file().sync_all()?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}
