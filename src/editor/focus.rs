use ratatui::layout::Rect;
/// Select the closest pane in the requested half-plane, preferring overlap.
pub fn neighbor(panes: &[Rect], current: usize, direction: char) -> usize {
    let Some(from) = panes.get(current) else {
        return current;
    };
    let (cx, cy) = (
        i32::from(from.x) * 2 + i32::from(from.width),
        i32::from(from.y) * 2 + i32::from(from.height),
    );
    panes
        .iter()
        .enumerate()
        .filter(|(i, r)| *i != current && r.width > 0 && r.height > 0)
        .filter_map(|(i, r)| {
            let (x, y) = (
                i32::from(r.x) * 2 + i32::from(r.width),
                i32::from(r.y) * 2 + i32::from(r.height),
            );
            let (along, cross, overlap) = match direction {
                'h' if x < cx => (
                    cx - x,
                    (cy - y).abs(),
                    from.y < r.bottom() && r.y < from.bottom(),
                ),
                'l' if x > cx => (
                    x - cx,
                    (cy - y).abs(),
                    from.y < r.bottom() && r.y < from.bottom(),
                ),
                'k' if y < cy => (
                    cy - y,
                    (cx - x).abs(),
                    from.x < r.right() && r.x < from.right(),
                ),
                'j' if y > cy => (
                    y - cy,
                    (cx - x).abs(),
                    from.x < r.right() && r.x < from.right(),
                ),
                _ => return None,
            };
            if !overlap {
                return None;
            }
            Some((i, along + cross))
        })
        .min_by_key(|(_, score)| *score)
        .map_or(current, |(i, _)| i)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tree_recent_source_pdf() {
        let p = [
            Rect::new(0, 0, 20, 20),
            Rect::new(0, 20, 20, 10),
            Rect::new(20, 0, 40, 30),
            Rect::new(60, 0, 40, 30),
        ];
        assert_eq!(neighbor(&p, 0, 'j'), 1);
        assert_eq!(neighbor(&p, 0, 'l'), 2);
        assert_eq!(neighbor(&p, 2, 'l'), 3);
        assert_eq!(neighbor(&p, 3, 'h'), 2);
        assert_eq!(neighbor(&p, 2, 'h'), 0);
        assert_eq!(neighbor(&p, 2, 'j'), 2);
    }
}
