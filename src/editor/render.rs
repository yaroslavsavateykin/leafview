use super::Editor;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthChar;
impl Editor {
    pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
        let name = self
            .buffers
            .get(self.current)
            .map(|b| {
                format!(
                    "{}{}",
                    b.path.file_name().unwrap_or_default().to_string_lossy(),
                    if b.dirty() { " [+]" } else { "" }
                )
            })
            .unwrap_or_else(|| "Source".into());
        let background = if self.focused {
            Color::Rgb(25, 32, 43)
        } else {
            Color::Rgb(20, 25, 33)
        };
        f.render_widget(Block::new().style(Style::default().bg(background)), area);
        let block =
            crate::files::panel_block(&name, self.focused).style(Style::default().bg(background));
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let Some(b) = self.buffers.get_mut(self.current) else {
            f.render_widget(Paragraph::new(self.message.clone()), inner);
            return;
        };
        let height = inner.height.saturating_sub(1) as usize;
        let cursor = b.cursor();
        let row = b.line();
        if row < b.scroll {
            b.scroll = row;
        }
        if row >= b.scroll + height.max(1) {
            b.scroll = row.saturating_sub(height.saturating_sub(1));
        }
        let selection = self.vim.selection(b);
        for (screen, line_no) in (b.scroll..b.state.doc.len_lines()).take(height).enumerate() {
            let start = b.start(line_no);
            let line = b.state.doc.line(line_no);
            let line_end = b.end(line_no);
            let tokens = b.syntax.line(
                &b.path,
                b.version,
                line_no,
                line,
                usize::from(inner.width).saturating_mul(4),
            );
            let mut spans = vec![Span::styled(
                format!("{:>4} ", line_no + 1),
                Style::default().fg(if line_no == row && self.focused {
                    Color::Rgb(125, 220, 240)
                } else {
                    Color::Rgb(117, 136, 157)
                }),
            )];
            let mut width = 5;
            for (offset, ch) in line.chars().enumerate() {
                if matches!(ch, '\r' | '\n') {
                    break;
                }
                let w = if ch == '\t' {
                    4
                } else {
                    ch.width().unwrap_or(0)
                };
                if width + w > inner.width as usize {
                    break;
                }
                let mut style = Style::default().fg(tokens
                    .get(offset)
                    .copied()
                    .unwrap_or(super::syntax::Token::Plain)
                    .color());
                if line_no == row && self.focused {
                    style = style.bg(Color::Rgb(33, 45, 60));
                }
                let pos = start + offset;
                if selection.is_some_and(|(a, z)| pos >= a && pos < z) {
                    style = style.bg(Color::Rgb(50, 75, 90));
                }
                if pos == cursor && self.focused {
                    style = style.bg(Color::Rgb(110, 205, 192)).fg(Color::Black);
                }
                let text = if ch == '\t' {
                    "    ".into()
                } else if ch.is_control() {
                    "�".into()
                } else {
                    ch.to_string()
                };
                spans.push(Span::styled(text, style));
                width += w;
            }
            if cursor == line_end && self.focused && width < inner.width as usize {
                spans.push(Span::styled(
                    " ",
                    Style::default().bg(Color::Rgb(110, 205, 192)),
                ));
            }
            f.render_widget(
                Paragraph::new(Line::from(spans)).style(Style::default().bg(
                    if line_no == row && self.focused {
                        Color::Rgb(33, 45, 60)
                    } else {
                        background
                    },
                )),
                Rect::new(inner.x, inner.y + screen as u16, inner.width, 1),
            );
        }
        let status = if self.vim.mode == super::vim::Mode::Command {
            self.vim.command.as_str()
        } else if !self.message.is_empty() {
            &self.message
        } else {
            &self.vim.message
        };
        let status = format!(
            " {:?}  {}:{}  {}",
            self.vim.mode,
            row + 1,
            cursor - b.start(row) + 1,
            crate::links::sanitize(status)
        );
        f.render_widget(
            Paragraph::new(status).style(
                Style::default()
                    .fg(if self.focused {
                        Color::Rgb(155, 235, 225)
                    } else {
                        Color::Rgb(150, 166, 185)
                    })
                    .bg(Color::Rgb(35, 47, 62)),
            ),
            Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
        );
    }
}
