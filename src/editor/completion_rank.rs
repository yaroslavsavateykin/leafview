//! Filter before truncation. Servers (notably tinymist) return unfiltered lists.
use super::{completion, Buffer};
use serde_json::Value;
pub(super) fn rank(items: Vec<Value>, b: &Buffer) -> Vec<Value> {
    let mut ranked = Vec::new();
    for (order, item) in items.into_iter().take(20000).enumerate() {
        let (start, _, _) = completion::edit(&item, b);
        let cursor = b.cursor();
        let query = if start <= cursor && cursor - start <= 512 {
            b.state.doc.slice(start..cursor).to_string()
        } else {
            String::new()
        };
        let label = item["filterText"].as_str().unwrap_or_else(|| {
            item["label"]
                .as_str()
                .unwrap_or("")
                .split(['(', ' '])
                .next()
                .unwrap_or("")
        });
        let label = label.chars().take(1024).collect::<String>();
        let query = query.trim_start_matches('#');
        let score = if query.is_empty() {
            0
        } else {
            let matches = helix_core::fuzzy::fuzzy_match(query, [label.as_str()], false);
            let Some((_, score)) = matches.first() else {
                continue;
            };
            let lower = label.to_lowercase();
            let q = query.to_lowercase();
            u32::from(*score)
                + if lower == q {
                    200000
                } else if lower.starts_with(&q) {
                    100000
                } else {
                    0
                }
        };
        let sort = item["sortText"].as_str().unwrap_or(&label).to_owned();
        ranked.push((score, sort, order, item));
    }
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    ranked
        .into_iter()
        .take(200)
        .map(|(_, _, _, item)| item)
        .collect()
}
