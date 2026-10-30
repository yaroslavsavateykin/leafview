//! Quiet, bounded welcome screen. No placeholder PDF or wrapped path walls.
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};
use std::path::{Path, PathBuf};
const BG: Color = Color::Rgb(20, 22, 26);
const INK: Color = Color::Rgb(210, 219, 225);
const MUTED: Color = Color::Rgb(115, 129, 140);
const ACCENT: Color = Color::Rgb(140, 200, 175);

fn compact(path: &Path, width: usize) -> String {
    let text = crate::links::sanitize(&path.display().to_string());
    let text = std::env::var("HOME")
        .ok()
        .filter(|h| text == *h || text.starts_with(&format!("{h}/")))
        .map(|h| format!("~{}", &text[h.len()..]))
        .unwrap_or(text);
    if text.chars().count() <= width {
        text
    } else {
        format!(
            "…{}",
            text.chars()
                .rev()
                .take(width.saturating_sub(1))
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>()
        )
    }
}

pub fn hit(area: Rect, len: usize, x: u16, y: u16) -> Option<usize> {
    if area.width < 12 || area.height < 5 {
        return None;
    }
    let width = area.width.saturating_sub(8).min(68);
    let count = len
        .min(9)
        .min(usize::from(area.height.saturating_sub(13) / 3));
    let height = (11 + count as u16 * 3).min(area.height);
    let left = area.x + (area.width - width) / 2;
    let top = area.y + area.height.saturating_sub(height) / 3 + 8;
    if x < left || x >= left + width || y < top {
        return None;
    }
    let index = usize::from((y - top) / 3);
    (index < count && (y - top) % 3 < 2).then_some(index)
}

pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    folder: &Path,
    recent: &[(PathBuf, PathBuf)],
    hovered: Option<usize>,
) {
    frame.render_widget(Block::new().style(Style::default().bg(BG)), area);
    if area.width < 12 || area.height < 5 {
        return;
    }
    let width = area.width.saturating_sub(8).min(68);
    let count = recent
        .len()
        .min(9)
        .min(usize::from(area.height.saturating_sub(13) / 3));
    let height = (11 + count as u16 * 3).min(area.height);
    let body = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + area.height.saturating_sub(height) / 3,
        width,
        height,
    );
    let muted = Style::default().fg(MUTED);
    let mut lines = vec![
        Line::styled(
            "LeafView",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Line::styled("A quiet place for your documents.", muted),
        Line::from(""),
        Line::styled("No document open", Style::default().fg(INK)),
        Line::styled(
            format!(
                "Browse  {}",
                compact(folder, usize::from(width.saturating_sub(8)))
            ),
            muted,
        ),
        Line::from(""),
        Line::styled("RECENT WORKSPACES", muted),
        Line::from(""),
    ];
    if recent.is_empty() {
        lines.push(Line::styled("Your next document starts here.", muted));
        lines.push(Line::styled(
            "Choose a file from the browser to get started.",
            muted,
        ));
    }
    for (i, (root, document)) in recent.iter().take(count).enumerate() {
        let name = document.file_name().map(Path::new).unwrap_or(document);
        lines.push(Line::from(vec![
            Span::styled(format!("{}   ", i + 1), Style::default().fg(ACCENT)),
            Span::styled(
                compact(name, usize::from(width.saturating_sub(4))),
                Style::default().fg(INK),
            ),
        ]));
        lines.push(Line::styled(
            format!(
                "    {}",
                compact(root, usize::from(width.saturating_sub(4)))
            ),
            muted,
        ));
        lines.push(Line::from(""));
    }
    if let Some(index) = hovered.filter(|i| *i < count) {
        for line in lines.iter_mut().skip(8 + index * 3).take(2) {
            *line = line
                .clone()
                .style(Style::default().bg(Color::Rgb(35, 55, 54)));
        }
    }
    frame.render_widget(Paragraph::new(lines), body);
    let footer = Rect::new(body.x, area.bottom().saturating_sub(2), width, 1);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("1–9", Style::default().fg(ACCENT)),
            Span::styled(" open   ", muted),
            Span::styled("Tab", Style::default().fg(INK)),
            Span::styled(" focus   ", muted),
            Span::styled("F2", Style::default().fg(INK)),
            Span::styled(" files", muted),
        ])),
        footer,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_hit_matches_rendered_rows() {
        let area = ratatui::layout::Rect::new(30, 0, 101, 40);
        assert_eq!(super::hit(area, 9, 47, 8), Some(0));
        assert_eq!(super::hit(area, 9, 47, 9), Some(0));
        assert_eq!(super::hit(area, 9, 47, 10), None);
        assert_eq!(super::hit(area, 9, 47, 32), Some(8));
        assert_eq!(super::hit(area, 9, 0, 8), None);
    }
    #[test]
    fn welcome_is_english_bounded_and_renders_tiny_sizes() {
        for (w, h) in [(0, 0), (1, 1), (41, 12), (101, 40)] {
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            let recent = (0..9).map(|i| (std::path::PathBuf::from("/home/research/projects/a-very-long-project-directory/manuscript"), std::path::PathBuf::from(format!("/home/research/projects/a-very-long-project-directory/manuscript/document-{i}.typ")))).collect::<Vec<_>>();
            terminal
                .draw(|f| super::render(f, f.area(), std::path::Path::new("/work"), &recent, None))
                .unwrap();
            if w == 101 {
                let rows = terminal
                    .backend()
                    .buffer()
                    .content
                    .chunks(w as usize)
                    .map(|r| r.iter().map(|c| c.symbol()).collect::<String>())
                    .collect::<Vec<_>>();
                let output = rows.join("\n");
                assert!(output.contains("No document open"));
                assert!(output.contains("document-8.typ"));
                assert!(!output
                    .chars()
                    .any(|c| ('\u{0400}'..='\u{04ff}').contains(&c)));
                std::fs::write("/tmp/leafview-welcome-render.txt", output).unwrap();
            }
        }
    }
}
