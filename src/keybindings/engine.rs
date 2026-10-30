//! Compiled context tries. No command-table search on the key dispatch path.
use super::{keys::Stroke, registry::Invocation};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
#[derive(Clone, Debug)]
pub struct Binding {
    pub context: String,
    pub mode: String,
    pub keys: Vec<Stroke>,
    pub invocation: Option<Invocation>,
    pub priority: i32,
    pub source: String,
}
#[derive(Default)]
struct Node {
    children: BTreeMap<Stroke, usize>,
    binding: Option<usize>,
}
#[derive(Default)]
struct Trie {
    nodes: Vec<Node>,
}
#[derive(Clone, Debug)]
pub struct Conflict {
    pub winner: usize,
    pub loser: usize,
    pub prefix: bool,
}
#[derive(Debug)]
pub enum Resolution {
    Pass,
    Pending,
    Consumed,
    Execute(Invocation),
    Replay(Vec<Stroke>),
}
pub struct Engine {
    pub bindings: Vec<Binding>,
    pub conflicts: Vec<Conflict>,
    tries: BTreeMap<(String, String), Trie>,
    active: Option<((String, String), usize)>,
    pending: Vec<Stroke>,
    deferred: Option<Invocation>,
    deadline: Option<Instant>,
    pub timeout: Duration,
    count: usize,
}
impl Engine {
    pub fn compile(bindings: Vec<Binding>, timeout_ms: u64) -> Self {
        let mut engine = Self {
            bindings,
            conflicts: vec![],
            tries: BTreeMap::new(),
            active: None,
            pending: vec![],
            deferred: None,
            deadline: None,
            timeout: Duration::from_millis(timeout_ms),
            count: 0,
        };
        let mut order = (0..engine.bindings.len()).collect::<Vec<_>>();
        order.sort_by_key(|i| (engine.bindings[*i].priority, *i));
        for i in order {
            let b = &engine.bindings[i];
            let trie = engine
                .tries
                .entry((b.context.clone(), b.mode.clone()))
                .or_insert_with(|| Trie {
                    nodes: vec![Node::default()],
                });
            let mut node = 0;
            for key in &b.keys {
                node = if let Some(n) = trie.nodes[node].children.get(key) {
                    *n
                } else {
                    let n = trie.nodes.len();
                    trie.nodes.push(Node::default());
                    trie.nodes[node].children.insert(key.clone(), n);
                    n
                };
            }
            if let Some(loser) = trie.nodes[node].binding.replace(i) {
                engine.conflicts.push(Conflict {
                    winner: i,
                    loser,
                    prefix: false,
                });
            }
        }
        for trie in engine.tries.values() {
            for node in &trie.nodes {
                if let Some(winner) = node.binding {
                    for child in node.children.values() {
                        if let Some(loser) = trie.nodes[*child].binding {
                            engine.conflicts.push(Conflict {
                                winner,
                                loser,
                                prefix: true,
                            });
                        }
                    }
                }
            }
        }
        engine
    }
    pub fn reset(&mut self) {
        self.active = None;
        self.pending.clear();
        self.deferred = None;
        self.deadline = None;
        self.count = 0;
    }
    pub fn waiting(&self) -> bool {
        self.active.is_some()
    }
    /// Return only effective terminal bindings, excluding shadowed layer entries.
    pub fn effective(&self) -> Vec<&Binding> {
        self.tries
            .values()
            .flat_map(|t| {
                t.nodes
                    .iter()
                    .filter_map(|n| n.binding.map(|i| &self.bindings[i]))
            })
            .collect()
    }
    pub fn prefix(&self) -> String {
        self.pending
            .iter()
            .map(Stroke::label)
            .collect::<Vec<_>>()
            .join(" → ")
    }
    pub fn hints(&self) -> Vec<(String, String)> {
        let Some((root, node)) = &self.active else {
            return vec![];
        };
        let trie = &self.tries[root];
        trie.nodes[*node]
            .children
            .iter()
            .map(|(key, n)| {
                let command = trie.nodes[*n]
                    .binding
                    .and_then(|i| self.bindings[i].invocation.as_ref())
                    .map(|v| v.command.clone())
                    .unwrap_or_else(|| "Group".into());
                (key.label(), command)
            })
            .collect()
    }
    pub fn expire(&mut self, now: Instant) -> Option<Resolution> {
        if self.deadline.is_none_or(|due| now < due) {
            return None;
        }
        let result = if let Some(mut command) = self.deferred.take() {
            command.count = self.count.max(1);
            Resolution::Execute(command)
        } else {
            Resolution::Replay(std::mem::take(&mut self.pending))
        };
        self.reset();
        Some(result)
    }
    pub fn feed(&mut self, key: Stroke, contexts: &[&str], mode: &str, now: Instant) -> Resolution {
        if !self.pending.is_empty() && key.key == "Esc" {
            self.reset();
            return Resolution::Consumed;
        }
        if let Some((root, node)) = &self.active {
            let root = root.clone();
            let node = *node;
            if let Some(next) = self.tries[&root].nodes[node].children.get(&key).copied() {
                self.pending.push(key);
                return self.arrive(root, next, now);
            }
            let mut pending = std::mem::take(&mut self.pending);
            pending.push(key);
            self.reset();
            return Resolution::Replay(pending);
        }
        if mode == "normal"
            && key.modifiers == 0
            && key.key.len() == 1
            && key.key.as_bytes()[0].is_ascii_digit()
            && (self.count > 0 || key.key != "0")
        {
            self.count = self
                .count
                .saturating_mul(10)
                .saturating_add((key.key.as_bytes()[0] - b'0') as usize)
                .min(100000);
            self.pending.push(key);
            self.deadline = Some(now + self.timeout);
            return Resolution::Pending;
        }
        for context in contexts {
            for candidate_mode in [mode, "any"] {
                let root = (context.to_string(), candidate_mode.to_string());
                if let Some(next) = self
                    .tries
                    .get(&root)
                    .and_then(|t| t.nodes[0].children.get(&key))
                    .copied()
                {
                    self.pending.push(key);
                    return self.arrive(root, next, now);
                }
            }
        }
        if !self.pending.is_empty() {
            let mut keys = std::mem::take(&mut self.pending);
            keys.push(key);
            self.reset();
            return Resolution::Replay(keys);
        }
        Resolution::Pass
    }
    fn arrive(&mut self, root: (String, String), node: usize, now: Instant) -> Resolution {
        let trie = &self.tries[&root];
        let binding = trie.nodes[node].binding;
        if trie.nodes[node].children.is_empty() {
            let command = binding.and_then(|i| self.bindings[i].invocation.clone());
            let count = self.count.max(1);
            self.reset();
            return match command {
                Some(mut c) => {
                    c.count = count;
                    Resolution::Execute(c)
                }
                None => Resolution::Consumed,
            };
        }
        self.deferred = binding.and_then(|i| self.bindings[i].invocation.clone());
        self.active = Some((root, node));
        self.deadline = Some(now + self.timeout);
        Resolution::Pending
    }
}
