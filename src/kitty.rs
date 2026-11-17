//! Lossless PNG Kitty transport. One bounded transfer, followed by reusable
//! Unicode placements. IDs remain process-owned so cleanup cannot erase a pane.
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    DynamicImage, ImageEncoder,
};
use ratatui::{
    buffer::{Buffer, CellDiffOption},
    layout::{Rect, Size},
    widgets::Widget,
};
use std::{
    fmt::Write,
    num::NonZeroU16,
    sync::atomic::{AtomicU64, Ordering},
};

static TERMINAL_EPOCH: AtomicU64 = AtomicU64::new(1);
static SLOTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<u32, std::sync::Weak<AtomicU64>>>,
> = std::sync::OnceLock::new();

fn delete_image(writer: &mut impl std::io::Write, id: u32, tmux: bool) -> std::io::Result<()> {
    let command = format!("\x1b_Ga=d,d=I,i={id},q=2;\x1b\\");
    if tmux {
        write!(
            writer,
            "\x1bPtmux;{}\x1b\\",
            command.replace('\x1b', "\x1b\x1b")
        )
    } else {
        writer.write_all(command.as_bytes())
    }
}

pub fn cleanup(writer: &mut impl std::io::Write, tmux: bool) -> std::io::Result<()> {
    let mut slots = SLOTS.get_or_init(Default::default).lock().unwrap();
    for &id in slots.keys() {
        delete_image(writer, id, tmux)?;
    }
    slots.clear();
    writer.flush()
}

/// A new SSH/tmux client has no copies of the old client's image cache.
pub fn invalidate_terminal_cache() {
    TERMINAL_EPOCH.fetch_add(1, Ordering::SeqCst);
}

pub struct KittyImage {
    pub size: Size,
    id: u32,
    transfer: std::sync::Arc<str>,
    tmux: bool,
    sent: std::sync::Arc<AtomicU64>,
    diff_color: ratatui::style::Color,
}
impl KittyImage {
    pub fn encoded_len(&self) -> usize {
        self.transfer.len()
    }
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn lease(&self) -> std::sync::Weak<AtomicU64> {
        std::sync::Arc::downgrade(&self.sent)
    }
    // Reuse the terminal-resident image, not just its encoded PNG. All clones
    // share upload state; a never-displayed queued image still uploads once.
    pub fn replay(&self) -> Self {
        Self {
            size: self.size,
            id: self.id,
            transfer: self.transfer.clone(),
            tmux: self.tmux,
            sent: self.sent.clone(),
            diff_color: self.diff_color,
        }
    }
    /// Upload outside Ratatui's diff buffer: an overlay can replace image cells
    /// before flush, so rendering a widget cannot mean the transfer was sent.
    pub fn upload(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        let mut slots = SLOTS.get_or_init(Default::default).lock().unwrap();
        let epoch = TERMINAL_EPOCH.load(Ordering::SeqCst);
        let same = slots
            .get(&self.id)
            .and_then(std::sync::Weak::upgrade)
            .is_some_and(|owner| std::sync::Arc::ptr_eq(&owner, &self.sent));
        if same && self.sent.load(Ordering::SeqCst) == epoch {
            return Ok(());
        }
        // Release only images no longer held by cache, visible frame or worker.
        let retired: Vec<_> = slots
            .iter()
            .filter(|(_, owner)| owner.strong_count() == 0)
            .map(|(&id, _)| id)
            .collect();
        for id in retired {
            delete_image(writer, id, self.tmux)?;
            slots.remove(&id);
        }
        // Replacing image data alone can retain an old virtual placement's
        // dimensions/content in terminals. Remove this owned slot first so
        // zoom/page cache reuse creates a fresh placement, not a stale one.
        let delete = format!("\x1b_Ga=d,d=I,i={},q=2;\x1b\\", self.id);
        if self.tmux {
            write!(
                writer,
                "\x1bPtmux;{}\x1b\\",
                delete.replace('\x1b', "\x1b\x1b")
            )?;
        } else {
            writer.write_all(delete.as_bytes())?;
        }
        writer.write_all(self.transfer.as_bytes())?;
        writer.flush()?;
        self.sent.store(epoch, Ordering::SeqCst);
        slots.retain(|_, owner| owner.strong_count() > 0);
        slots.insert(self.id, std::sync::Arc::downgrade(&self.sent));
        Ok(())
    }
    pub fn new(image: DynamicImage, size: Size, id: u32, tmux: bool) -> Result<Self> {
        let rgb = image.into_rgba8();
        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Sub)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgba8,
            )?;
        static NEXT_CONTENT: AtomicU64 = AtomicU64::new(1);
        let [_, r, g, b] = (NEXT_CONTENT.fetch_add(1, Ordering::Relaxed) as u32).to_be_bytes();
        let transfer = transfer(&png, id, size, tmux);
        Ok(Self {
            size,
            id,
            transfer: transfer.into(),
            tmux,
            sent: std::sync::Arc::new(AtomicU64::new(0)),
            diff_color: ratatui::style::Color::Rgb(r, g, b),
        })
    }
}
fn transfer(png: &[u8], id: u32, size: Size, tmux: bool) -> String {
    let mut out = String::new();
    let count = png.len().div_ceil(3072);
    for (i, chunk) in png.chunks(3072).enumerate() {
        let mut command = String::from("\x1b_Gq=2,");
        if i == 0 {
            // The negotiated font metrics can differ from the outer terminal
            // (especially across tmux). Define the virtual placement grid
            // explicitly: placeholders use these rows/columns, not PNG pixels.
            write!(
                command,
                "i={id},a=T,U=1,f=100,t=d,c={},r={},",
                size.width, size.height
            )
            .unwrap();
        }
        write!(command, "m={};", u8::from(i + 1 < count)).unwrap();
        STANDARD.encode_string(chunk, &mut command);
        command.push_str("\x1b\\");
        if tmux {
            out.push_str("\x1bPtmux;");
            out.push_str(&command.replace('\x1b', "\x1b\x1b"));
            out.push_str("\x1b\\");
        } else {
            out.push_str(&command);
        }
    }
    out
}
impl Widget for &KittyImage {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = area.width.min(self.size.width).min(DIACRITICS.len() as u16);
        let height = area
            .height
            .min(self.size.height)
            .min(DIACRITICS.len() as u16);
        if width == 0 || height == 0 {
            return;
        }
        let [extra, r, g, b] = self.id.to_be_bytes();

        for y in 0..height {
            for x in 0..width {
                // Each placeholder must own exactly one buffer cell. A whole
                // row stored in its first cell makes Ratatui skip overlays in
                // the middle of that row (even after Clear resets their cells).
                // Explicit coordinates also let isolated cells be restored
                // correctly when an overlay closes. DECRC restores the cursor
                // as well as the foreground, so advance it again: Crossterm
                // omits MoveTo for adjacent cells and expects Print to advance
                // by one column. CUF clamps at the right margin without wrap.
                let symbol = format!(
                    "\x1b7\x1b[?7l\x1b[38;2;{r};{g};{b}m\u{10eeee}{}{}{}\x1b[?7h\x1b8\x1b[1C",
                    DIACRITICS[y as usize], DIACRITICS[x as usize], DIACRITICS[extra as usize]
                );
                buf[(area.x + x, area.y + y)]
                    .set_symbol(&symbol)
                    .set_fg(self.diff_color)
                    .set_diff_option(CellDiffOption::ForcedWidth(NonZeroU16::new(1).unwrap()));
            }
        }
    }
}

// Kitty protocol row/column diacritics; table from ratatui-image (MIT).
static DIACRITICS: [char; 297] = [
    '\u{305}',
    '\u{30D}',
    '\u{30E}',
    '\u{310}',
    '\u{312}',
    '\u{33D}',
    '\u{33E}',
    '\u{33F}',
    '\u{346}',
    '\u{34A}',
    '\u{34B}',
    '\u{34C}',
    '\u{350}',
    '\u{351}',
    '\u{352}',
    '\u{357}',
    '\u{35B}',
    '\u{363}',
    '\u{364}',
    '\u{365}',
    '\u{366}',
    '\u{367}',
    '\u{368}',
    '\u{369}',
    '\u{36A}',
    '\u{36B}',
    '\u{36C}',
    '\u{36D}',
    '\u{36E}',
    '\u{36F}',
    '\u{483}',
    '\u{484}',
    '\u{485}',
    '\u{486}',
    '\u{487}',
    '\u{592}',
    '\u{593}',
    '\u{594}',
    '\u{595}',
    '\u{597}',
    '\u{598}',
    '\u{599}',
    '\u{59C}',
    '\u{59D}',
    '\u{59E}',
    '\u{59F}',
    '\u{5A0}',
    '\u{5A1}',
    '\u{5A8}',
    '\u{5A9}',
    '\u{5AB}',
    '\u{5AC}',
    '\u{5AF}',
    '\u{5C4}',
    '\u{610}',
    '\u{611}',
    '\u{612}',
    '\u{613}',
    '\u{614}',
    '\u{615}',
    '\u{616}',
    '\u{617}',
    '\u{657}',
    '\u{658}',
    '\u{659}',
    '\u{65A}',
    '\u{65B}',
    '\u{65D}',
    '\u{65E}',
    '\u{6D6}',
    '\u{6D7}',
    '\u{6D8}',
    '\u{6D9}',
    '\u{6DA}',
    '\u{6DB}',
    '\u{6DC}',
    '\u{6DF}',
    '\u{6E0}',
    '\u{6E1}',
    '\u{6E2}',
    '\u{6E4}',
    '\u{6E7}',
    '\u{6E8}',
    '\u{6EB}',
    '\u{6EC}',
    '\u{730}',
    '\u{732}',
    '\u{733}',
    '\u{735}',
    '\u{736}',
    '\u{73A}',
    '\u{73D}',
    '\u{73F}',
    '\u{740}',
    '\u{741}',
    '\u{743}',
    '\u{745}',
    '\u{747}',
    '\u{749}',
    '\u{74A}',
    '\u{7EB}',
    '\u{7EC}',
    '\u{7ED}',
    '\u{7EE}',
    '\u{7EF}',
    '\u{7F0}',
    '\u{7F1}',
    '\u{7F3}',
    '\u{816}',
    '\u{817}',
    '\u{818}',
    '\u{819}',
    '\u{81B}',
    '\u{81C}',
    '\u{81D}',
    '\u{81E}',
    '\u{81F}',
    '\u{820}',
    '\u{821}',
    '\u{822}',
    '\u{823}',
    '\u{825}',
    '\u{826}',
    '\u{827}',
    '\u{829}',
    '\u{82A}',
    '\u{82B}',
    '\u{82C}',
    '\u{82D}',
    '\u{951}',
    '\u{953}',
    '\u{954}',
    '\u{F82}',
    '\u{F83}',
    '\u{F86}',
    '\u{F87}',
    '\u{135D}',
    '\u{135E}',
    '\u{135F}',
    '\u{17DD}',
    '\u{193A}',
    '\u{1A17}',
    '\u{1A75}',
    '\u{1A76}',
    '\u{1A77}',
    '\u{1A78}',
    '\u{1A79}',
    '\u{1A7A}',
    '\u{1A7B}',
    '\u{1A7C}',
    '\u{1B6B}',
    '\u{1B6D}',
    '\u{1B6E}',
    '\u{1B6F}',
    '\u{1B70}',
    '\u{1B71}',
    '\u{1B72}',
    '\u{1B73}',
    '\u{1CD0}',
    '\u{1CD1}',
    '\u{1CD2}',
    '\u{1CDA}',
    '\u{1CDB}',
    '\u{1CE0}',
    '\u{1DC0}',
    '\u{1DC1}',
    '\u{1DC3}',
    '\u{1DC4}',
    '\u{1DC5}',
    '\u{1DC6}',
    '\u{1DC7}',
    '\u{1DC8}',
    '\u{1DC9}',
    '\u{1DCB}',
    '\u{1DCC}',
    '\u{1DD1}',
    '\u{1DD2}',
    '\u{1DD3}',
    '\u{1DD4}',
    '\u{1DD5}',
    '\u{1DD6}',
    '\u{1DD7}',
    '\u{1DD8}',
    '\u{1DD9}',
    '\u{1DDA}',
    '\u{1DDB}',
    '\u{1DDC}',
    '\u{1DDD}',
    '\u{1DDE}',
    '\u{1DDF}',
    '\u{1DE0}',
    '\u{1DE1}',
    '\u{1DE2}',
    '\u{1DE3}',
    '\u{1DE4}',
    '\u{1DE5}',
    '\u{1DE6}',
    '\u{1DFE}',
    '\u{20D0}',
    '\u{20D1}',
    '\u{20D4}',
    '\u{20D5}',
    '\u{20D6}',
    '\u{20D7}',
    '\u{20DB}',
    '\u{20DC}',
    '\u{20E1}',
    '\u{20E7}',
    '\u{20E9}',
    '\u{20F0}',
    '\u{2CEF}',
    '\u{2CF0}',
    '\u{2CF1}',
    '\u{2DE0}',
    '\u{2DE1}',
    '\u{2DE2}',
    '\u{2DE3}',
    '\u{2DE4}',
    '\u{2DE5}',
    '\u{2DE6}',
    '\u{2DE7}',
    '\u{2DE8}',
    '\u{2DE9}',
    '\u{2DEA}',
    '\u{2DEB}',
    '\u{2DEC}',
    '\u{2DED}',
    '\u{2DEE}',
    '\u{2DEF}',
    '\u{2DF0}',
    '\u{2DF1}',
    '\u{2DF2}',
    '\u{2DF3}',
    '\u{2DF4}',
    '\u{2DF5}',
    '\u{2DF6}',
    '\u{2DF7}',
    '\u{2DF8}',
    '\u{2DF9}',
    '\u{2DFA}',
    '\u{2DFB}',
    '\u{2DFC}',
    '\u{2DFD}',
    '\u{2DFE}',
    '\u{2DFF}',
    '\u{A66F}',
    '\u{A67C}',
    '\u{A67D}',
    '\u{A6F0}',
    '\u{A6F1}',
    '\u{A8E0}',
    '\u{A8E1}',
    '\u{A8E2}',
    '\u{A8E3}',
    '\u{A8E4}',
    '\u{A8E5}',
    '\u{A8E6}',
    '\u{A8E7}',
    '\u{A8E8}',
    '\u{A8E9}',
    '\u{A8EA}',
    '\u{A8EB}',
    '\u{A8EC}',
    '\u{A8ED}',
    '\u{A8EE}',
    '\u{A8EF}',
    '\u{A8F0}',
    '\u{A8F1}',
    '\u{AAB0}',
    '\u{AAB2}',
    '\u{AAB3}',
    '\u{AAB7}',
    '\u{AAB8}',
    '\u{AABE}',
    '\u{AABF}',
    '\u{AAC1}',
    '\u{FE20}',
    '\u{FE21}',
    '\u{FE22}',
    '\u{FE23}',
    '\u{FE24}',
    '\u{FE25}',
    '\u{FE26}',
    '\u{10A0F}',
    '\u{10A38}',
    '\u{1D185}',
    '\u{1D186}',
    '\u{1D187}',
    '\u{1D188}',
    '\u{1D189}',
    '\u{1D1AA}',
    '\u{1D1AB}',
    '\u{1D1AC}',
    '\u{1D1AD}',
    '\u{1D242}',
    '\u{1D243}',
    '\u{1D244}',
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adjacent_placeholders_advance_the_terminal_cursor() {
        use ratatui::{backend::Backend, backend::CrosstermBackend};

        let area = Rect::new(2, 1, 3, 1);
        let kitty =
            KittyImage::new(DynamicImage::new_rgb8(30, 10), Size::new(3, 1), 127, false).unwrap();
        let mut buf = Buffer::empty(area);
        (&kitty).render(area, &mut buf);
        let mut bytes = Vec::new();
        let mut backend = CrosstermBackend::new(&mut bytes);
        backend
            .draw((area.x..area.right()).map(|x| (x, area.y, &buf[(x, area.y)])))
            .unwrap();
        let wire = String::from_utf8(bytes).unwrap();
        // The backend positions only the first cell. Each save/restore pair
        // must therefore advance once or all three columns overwrite it.
        assert_eq!(wire.matches("\x1b[2;3H").count(), 1);
        assert_eq!(wire.matches("\x1b8\x1b[1C").count(), 3);
        assert_eq!(wire.matches('\u{10eeee}').count(), 3);
    }

    #[test]
    fn overlays_replace_image_cells_and_closing_restores_them() {
        use ratatui::widgets::{Clear, Paragraph};

        let area = Rect::new(0, 0, 12, 4);
        let image_area = Rect::new(1, 1, 10, 2);
        let overlay = Rect::new(4, 1, 4, 2);
        let kitty = KittyImage::new(
            DynamicImage::new_rgb8(100, 40),
            Size::new(10, 2),
            126,
            false,
        )
        .unwrap();
        let mut image = Buffer::empty(area);
        (&kitty).render(image_area, &mut image);
        let mut menu = image.clone();
        Clear.render(overlay, &mut menu);
        Paragraph::new("MENU").render(overlay, &mut menu);

        // Exercise the real diff: a wide image cell used to suppress all
        // updates inside the menu, although the final buffer contained it.
        let opening = image.diff(&menu);
        for y in overlay.y..overlay.bottom() {
            for x in overlay.x..overlay.right() {
                assert!(opening.iter().any(|(cx, cy, _)| (*cx, *cy) == (x, y)));
            }
        }
        let closing = menu.diff(&image);
        for y in overlay.y..overlay.bottom() {
            for x in overlay.x..overlay.right() {
                let (_, _, cell) = closing
                    .iter()
                    .find(|(cx, cy, _)| (*cx, *cy) == (x, y))
                    .expect("closing overlay must restore each image cell");
                assert!(cell.symbol().contains('\u{10eeee}'));
                let coordinates = format!(
                    "{}{}{}",
                    DIACRITICS[(y - image_area.y) as usize],
                    DIACRITICS[(x - image_area.x) as usize],
                    DIACRITICS[0]
                );
                assert!(cell.symbol().contains(&coordinates));
            }
        }
    }

    #[test]
    fn lossless_chunked_transport() {
        let image = DynamicImage::new_rgba8(400, 300);
        let kitty = KittyImage::new(image.clone(), Size::new(40, 15), 123, false).unwrap();
        let mut bytes = Vec::new();
        for command in kitty.transfer.split("\x1b_G").skip(1) {
            let (header, data) = command.split_once(';').unwrap();
            let data = data.strip_suffix("\x1b\\").unwrap();
            assert!(data.len() <= 4096);
            bytes.extend(STANDARD.decode(data).unwrap());
            assert!(header.contains("m="));
        }
        assert_eq!(
            image.to_rgba8(),
            image::load_from_memory(&bytes).unwrap().to_rgba8()
        );
        assert!(kitty.transfer.contains("f=100"));
        assert!(kitty.transfer.len() < 400 * 300);
        // Incompressible data exercises continuation framing, not just a tiny PNG.
        let payload: Vec<u8> = (0..20000).map(|i| (i * 37) as u8).collect();
        let commands = transfer(&payload, 123, Size::new(40, 15), false);
        let mut decoded = Vec::new();
        for command in commands.split("\x1b_G").skip(1) {
            let (_, data) = command.split_once(';').unwrap();
            let data = data.strip_suffix("\x1b\\").unwrap();
            assert!(data.len() <= 4096);
            decoded.extend(STANDARD.decode(data).unwrap());
        }
        assert_eq!(payload, decoded);
        assert_eq!(commands.matches("m=0;").count(), 1);
        assert_eq!(commands.matches("a=T").count(), 1);
        let replay = kitty.replay();
        let lease = kitty.lease();
        assert!(
            lease.strong_count() >= 2,
            "visible clones must protect their slot"
        );
        assert!(std::sync::Arc::ptr_eq(&kitty.transfer, &replay.transfer));
        assert_eq!(replay.encoded_len(), kitty.encoded_len());
        let wrapped = transfer(&bytes, 123, Size::new(40, 15), true);
        assert!(wrapped.contains("c=40,r=15,"));
        assert!(wrapped.starts_with("\x1bPtmux;\x1b\x1b_G"));
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 15));
        (&kitty).render(buf.area, &mut buf);
        assert!(!buf[(0, 0)].symbol().contains("f=100"));
        let mut wire = Vec::new();
        kitty.upload(&mut wire).unwrap();
        let commands = String::from_utf8_lossy(&wire);
        assert!(commands.contains("f=100"));
        assert!(commands.find("a=d,d=I,i=123").unwrap() < commands.find("a=T,U=1").unwrap());
        wire.clear();
        replay.upload(&mut wire).unwrap();
        assert!(wire.is_empty());
        assert!(buf[(0, 0)].symbol().contains("\x1b[?7l"));
        assert!(buf[(0, 0)].symbol().contains("\x1b[?7h"));
        assert_eq!(
            buf[(0, 0)].diff_option,
            CellDiffOption::ForcedWidth(NonZeroU16::new(1).unwrap())
        );
        (&kitty).render(buf.area, &mut buf);
        assert!(!buf[(0, 0)].symbol().contains("f=100"));
        let mut revisited = Buffer::empty(Rect::new(0, 0, 40, 15));
        (&replay).render(revisited.area, &mut revisited);
        assert!(
            !revisited[(0, 0)].symbol().contains("f=100"),
            "cached viewport uploaded PNG again"
        );
        // Simulate encoder slot reuse while an old visible/replay owner survives.
        let replacement = KittyImage::new(image.clone(), Size::new(40, 15), 123, false).unwrap();
        let previous = revisited.clone();
        (&replacement).render(revisited.area, &mut revisited);
        assert_eq!(
            previous.diff(&revisited).len(),
            40 * 15,
            "reused image slots must repaint without a terminal clear"
        );
        replacement.upload(&mut wire).unwrap();
        assert!(!wire.is_empty());
        wire.clear();
        (&replay).render(revisited.area, &mut revisited);
        replay.upload(&mut wire).unwrap();
        assert!(
            !wire.is_empty(),
            "old replay must restore content after slot reuse"
        );
        let fresh = KittyImage::new(image, Size::new(40, 15), 124, false).unwrap();
        let queued_clone = fresh.replay();
        let mut pending = Buffer::empty(Rect::new(0, 0, 40, 15));
        (&queued_clone).render(pending.area, &mut pending);
        // Entirely overwrite the widget before the frame is flushed. Upload
        // must still occur; widget rendering cannot mark the image as sent.
        pending.reset();
        wire.clear();
        queued_clone.upload(&mut wire).unwrap();
        assert!(!wire.is_empty(), "overlay lost the first upload");
        // Failed terminal IO must leave the transfer retryable.
        struct Failed;
        impl std::io::Write for Failed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let retry =
            KittyImage::new(DynamicImage::new_rgb8(2, 2), Size::new(1, 1), 125, false).unwrap();
        assert!(retry.upload(&mut Failed).is_err());
        wire.clear();
        retry.upload(&mut wire).unwrap();
        assert!(!wire.is_empty());
    }
}
