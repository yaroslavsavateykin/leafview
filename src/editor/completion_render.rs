use super::{completion, Editor};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
fn rgb(c: [u8; 3]) -> Color {
    Color::Rgb(c[0], c[1], c[2])
}
impl Editor {
    pub(super) fn draw_signature(&self, f: &mut Frame, area: Rect) {
        if self.windows.signature_hidden
            || area.width < 8
            || area.height < 4
            || !self.focused
            || self.vim.mode != super::vim::Mode::Insert
        {
            return;
        }
        let Some(b) = self.buffers.get(self.current) else {
            return;
        };
        if self.signature.path != b.path {
            return;
        }
        let Some(call) = super::signature::context_for(
            &b.state.doc,
            b.cursor(),
            b.path.extension().is_some_and(|s| s == "py"),
        ) else {
            return;
        };
        if self.signature.call.as_ref().map(|c| c.open) != Some(call.open) {
            return;
        }
        let Some((label, range, doc)) =
            super::signature::active_range(&self.signature.value, &call)
        else {
            return;
        };
        let c = &self.completion_config;
        let style = Style::default().fg(rgb(c.foreground)).bg(rgb(c.background));
        let active = style
            .fg(Color::Rgb(130, 210, 235))
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        let mut lines = vec![Line::from(vec![
            Span::raw(label[..range.start].to_owned()),
            Span::styled(label[range.clone()].to_owned(), active),
            Span::raw(label[range.end..].to_owned()),
        ])];
        lines.extend(markdown(&doc, &b.path));
        let width = (lines.iter().map(|l| l.width()).max().unwrap_or(20) as u16 + 2)
            .max(20)
            .min(c.documentation_width.max(20))
            .min(area.width);
        let wraps = lines
            .iter()
            .map(|l| {
                l.width()
                    .max(1)
                    .div_ceil(width.saturating_sub(2).max(1) as usize)
            })
            .sum::<usize>();
        let height = (wraps + 3).min(8).min((area.height as usize / 3).max(3)) as u16;
        let col: usize = b
            .state
            .doc
            .slice(b.start(b.line())..b.cursor())
            .chars()
            .map(|ch| {
                if ch == '\t' {
                    4
                } else {
                    ch.width().unwrap_or(0)
                }
            })
            .sum();
        let x = (area.x + 5)
            .saturating_add(col.min(u16::MAX as usize) as u16)
            .min(area.right() - width);
        let y = area
            .y
            .saturating_add(b.line().saturating_sub(b.scroll).min(u16::MAX as usize) as u16)
            .min(area.bottom() - 1);
        // Keep signature above the cursor, completion below. If no room above,
        // reserve a bottom strip rather than painting over completion.
        let y = if y >= area.y + height {
            y - height
        } else {
            let menu_height = if self.completion_visible() {
                self.popup.as_ref().map_or(0, |(_, items, _)| {
                    items.len().min(c.menu_height.max(1) as usize) as u16 + 2
                })
            } else {
                0
            };
            y.saturating_add(1 + menu_height)
                .min(area.bottom() - height)
        };
        let rect = Rect::new(x, y, width, height);
        f.render_widget(Clear, rect);
        let title = if call.argument > 0 {
            format!(" Arguments · {} · F8 × ", call.argument + 1)
        } else {
            " Signature · F7 focus · F8 × ".into()
        };
        f.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((
                    self.windows
                        .signature_scroll
                        .min(wraps.saturating_sub(height.saturating_sub(2) as usize) as u16),
                    0,
                ))
                .style(style)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_type(if c.rounded_border {
                            BorderType::Rounded
                        } else {
                            BorderType::Plain
                        })
                        .border_style(Style::default().fg(
                            if self.windows.focus == Some(super::windows::Window::Signature) {
                                Color::Rgb(125, 220, 240)
                            } else {
                                rgb(c.border_color)
                            },
                        )),
                ),
            rect,
        );
    }
    pub(super) fn draw_completion(&self, f: &mut Frame, area: Rect) {
        if area.width < 8 || area.height < 5 {
            return;
        }
        let Some((method, items, selected)) = &self.popup else {
            return;
        };
        let Some(b) = self.buffers.get(self.current) else {
            return;
        };
        let c = &self.completion_config;
        let style = Style::default().fg(rgb(c.foreground)).bg(rgb(c.background));
        let block = || {
            Block::default()
                .borders(Borders::ALL)
                .border_type(if c.rounded_border {
                    BorderType::Rounded
                } else {
                    BorderType::Plain
                })
                .border_style(Style::default().fg(rgb(c.border_color)))
                .style(style)
        };
        let col: usize = b
            .state
            .doc
            .slice(b.start(b.line())..b.cursor())
            .chars()
            .map(|ch| {
                if ch == '\t' {
                    4
                } else {
                    ch.width().unwrap_or(0)
                }
            })
            .sum();
        let x = area
            .x
            .saturating_add(5)
            .saturating_add(col.min(u16::MAX as usize) as u16)
            .min(area.right() - 3);
        let y = (area.y + b.line().saturating_sub(b.scroll) as u16).min(area.bottom() - 2);
        let is_completion = method == "textDocument/completion";
        let max_width = c.menu_max_width.max(8).min(area.width);
        let width = (items
            .iter()
            .map(|i| i["label"].as_str().unwrap_or("").width() + 8)
            .max()
            .unwrap_or(15) as u16)
            .max(c.menu_min_width)
            .min(max_width);
        let height = (items.len().min(c.menu_height.max(1) as usize) as u16 + 2).min(area.height);
        let south = area.bottom().saturating_sub(y + 1) >= height;
        let menu = Rect::new(
            x.min(area.right() - width),
            if south {
                y + 1
            } else {
                y.saturating_sub(height).max(area.y)
            },
            width,
            height,
        );
        if is_completion && c.ghost_text && self.cmp.selected && self.cmp.preview.is_none() {
            let (start, _, text) = completion::edit(&items[*selected], b);
            let prefix = if start <= b.cursor() {
                b.state.doc.slice(start..b.cursor()).to_string()
            } else {
                String::new()
            };
            if let Some(suffix) = text.strip_prefix(&prefix) {
                f.render_widget(
                    Paragraph::new(suffix.lines().next().unwrap_or("")).style(
                        Style::default()
                            .fg(Color::Rgb(80, 87, 99))
                            .add_modifier(Modifier::DIM),
                    ),
                    Rect::new(x, y, area.right().saturating_sub(x), 1),
                );
            }
        }
        if is_completion {
            f.render_widget(Clear, menu);
            f.render_widget(block(), menu);
            let inner = menu.inner(ratatui::layout::Margin::new(1, 1));
            let first = selected.saturating_sub(inner.height.saturating_sub(1) as usize);
            for (row, (i, item)) in items
                .iter()
                .enumerate()
                .skip(first)
                .take(inner.height as usize)
                .enumerate()
            {
                let active = self.cmp.selected && i == *selected;
                let (icon, _) = completion::kind(item["kind"].as_u64().unwrap_or(1));
                let label = crate::links::sanitize(item["label"].as_str().unwrap_or(""));
                let desc = crate::links::sanitize(
                    item["labelDetails"]["description"].as_str().unwrap_or(""),
                );
                let row_style = if active {
                    style
                        .bg(rgb(c.selected_background))
                        .add_modifier(Modifier::BOLD)
                } else {
                    style
                };
                let line = Line::from(vec![
                    Span::styled(
                        format!(" {icon} "),
                        Style::default().fg(Color::Rgb(130, 170, 255)),
                    ),
                    Span::raw(label),
                    Span::styled(
                        format!(" {desc}"),
                        Style::default().fg(Color::Rgb(95, 106, 125)),
                    ),
                ]);
                f.render_widget(
                    Paragraph::new(line).style(row_style),
                    Rect::new(inner.x, inner.y + row as u16, inner.width, 1),
                );
            }
            if items.len() > inner.height as usize && inner.height > 0 {
                let thumb = ((*selected * inner.height as usize) / items.len())
                    .min(inner.height as usize - 1) as u16;
                f.render_widget(
                    Paragraph::new("▐").style(Style::default().fg(rgb(c.border_color))),
                    Rect::new(menu.right() - 1, inner.y + thumb, 1, 1),
                );
            }
        }
        if is_completion && (!self.cmp.selected || self.cmp.documentation_hidden) {
            return;
        }
        let item = &items[*selected];
        let doc = if is_completion {
            completion::documentation(item)
        } else if method.ends_with("signatureHelp") {
            format!(
                "```\n{}\n```\n{}",
                item["label"].as_str().unwrap_or(""),
                completion::documentation(item)
            )
        } else {
            if method.ends_with("hover") {
                completion::documentation(item)
            } else {
                self.popup_text.join("\n")
            }
        };
        if doc.trim().is_empty() {
            return;
        }
        let east = area.right().saturating_sub(menu.right());
        let west = menu.x.saturating_sub(area.x);
        let desired = c.documentation_width.max(10).min(area.width);
        let lines = markdown(&doc, &b.path);
        let wanted_width = (lines
            .iter()
            .map(|line| line.width())
            .max()
            .unwrap_or(10)
            .min(u16::MAX as usize - 2) as u16
            + 2)
        .max(24)
        .min(desired);
        let desired = wanted_width;
        let wrapped = lines
            .iter()
            .map(|line| {
                line.width()
                    .max(1)
                    .div_ceil(desired.saturating_sub(2).max(1) as usize)
            })
            .sum::<usize>();
        let dh = (wrapped.min(u16::MAX as usize - 2) as u16 + 2)
            .max(3)
            .min(c.documentation_height.max(3))
            .min((area.height / 2).max(3));
        let doc_area = if is_completion && east >= 22 {
            Rect::new(
                menu.right(),
                menu.y.min(area.bottom() - dh),
                desired.min(east),
                dh,
            )
        } else if is_completion && west >= 22 {
            let dw = desired.min(west);
            Rect::new(menu.x - dw, menu.y.min(area.bottom() - dh), dw, dh)
        } else {
            let available = if south {
                area.bottom().saturating_sub(menu.bottom())
            } else {
                menu.y.saturating_sub(area.y)
            };
            if is_completion && available < 3 {
                return;
            }
            let h = if is_completion {
                dh.min(available)
            } else {
                dh.min(
                    y.saturating_sub(area.y)
                        .max(area.bottom().saturating_sub(y + 1)),
                )
                .max(3)
                .min(area.height)
            };
            Rect::new(
                x.min(area.right() - desired),
                if is_completion && south {
                    menu.bottom()
                } else if !is_completion
                    && y < area.y + h
                    && area.bottom().saturating_sub(y + 1) >= h
                {
                    y + 1
                } else {
                    y.saturating_sub(h).max(area.y)
                },
                desired,
                h,
            )
        };
        f.render_widget(Clear, doc_area);
        let wrapped = lines
            .iter()
            .map(|l| {
                l.width()
                    .max(1)
                    .div_ceil(doc_area.width.saturating_sub(2).max(1) as usize)
            })
            .sum::<usize>();
        let max_scroll = wrapped
            .saturating_sub(doc_area.height.saturating_sub(2) as usize)
            .min(u16::MAX as usize) as u16;
        let scroll = if is_completion {
            self.cmp.documentation_scroll
        } else {
            self.windows.hover_scroll
        };
        let title = if method.ends_with("hover") {
            " Hover · F7 focus · F8 × "
        } else if is_completion {
            " Documentation · F7 · F8 × "
        } else {
            " Locations · Enter · F8 × "
        };
        f.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((scroll.min(max_scroll), 0))
                .style(style)
                .block(
                    block()
                        .title(title)
                        .title_bottom(if wrapped > doc_area.height.saturating_sub(2) as usize {
                            " F7 focus · PgUp/PgDn · F8 × "
                        } else {
                            " F7 focus · F8 close "
                        })
                        .border_style(Style::default().fg(
                            if self.windows.focus == Some(super::windows::Window::Documentation) {
                                Color::Rgb(125, 220, 240)
                            } else {
                                rgb(c.border_color)
                            },
                        )),
                ),
            doc_area,
        );
    }
}
fn markdown(text: &str, path: &std::path::Path) -> Vec<Line<'static>> {
    let mut code = false;
    let mut language = path.to_owned();
    let mut cache = super::syntax::SyntaxCache::default();
    text.lines()
        .take(2000)
        .enumerate()
        .filter_map(|(row, line)| {
            if let Some(lang) = line.strip_prefix("```") {
                code = !code;
                language = match lang.trim() {
                    "rust" => "a.rs".into(),
                    "python" => "a.py".into(),
                    "typst" => "a.typ".into(),
                    _ => path.to_owned(),
                };
                return None;
            }
            let line = crate::links::sanitize(line);
            if code {
                let rope = helix_core::Rope::from_str(&line);
                let tokens = cache.line(&language, 0, row, rope.slice(..), 4096);
                Some(Line::from(
                    line.chars()
                        .enumerate()
                        .map(|(i, ch)| {
                            Span::styled(
                                ch.to_string(),
                                Style::default().fg(tokens
                                    .get(i)
                                    .copied()
                                    .unwrap_or(super::syntax::Token::Plain)
                                    .color()),
                            )
                        })
                        .collect::<Vec<_>>(),
                ))
            } else {
                let heading = line.starts_with('#');
                let mut bold = heading;
                let spans = line
                    .trim_start_matches('#')
                    .split("**")
                    .map(|part| {
                        let style = if bold {
                            Style::default().add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };
                        bold = !bold;
                        Span::styled(part.replace('`', ""), style)
                    })
                    .collect::<Vec<_>>();
                Some(Line::from(spans))
            }
        })
        .collect()
}
