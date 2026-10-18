//! UI-independent navigation and geometry (unit testable without a terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport { pub width: u32, pub height: u32 }

#[derive(Debug, Clone)]
pub struct View {
    pub page: u32,
    pub pages: u32,
    pub zoom: f32,
    /// Page-relative scroll in rendered pixels.
    pub x: u32,
    pub y: u32,
}

impl View {
    pub fn new(pages: u32) -> Self {
        Self { page: 0, pages: pages.max(1), zoom: 1.0, x: 0, y: 0 }
    }
    pub fn step_page(&mut self, delta: i32) -> bool {
        let next = (self.page as i64 + delta as i64).clamp(0, self.pages as i64 - 1) as u32;
        if next == self.page { return false; }
        self.page = next;
        self.x = 0; self.y = 0;
        true
    }
    pub fn go_to(&mut self, page_1_based: u32) -> bool {
        let next = page_1_based.saturating_sub(1).min(self.pages - 1);
        if next == self.page { return false; }
        self.page = next;
        self.x = 0; self.y = 0;
        true
    }
    pub fn adjust_zoom(&mut self, delta: f32) -> bool {
        let new = (self.zoom + delta).clamp(0.5, 3.0);
        if (new - self.zoom).abs() < 0.001 { return false; }
        self.x = (self.x as f32 * new / self.zoom) as u32;
        self.y = (self.y as f32 * new / self.zoom) as u32;
        self.zoom = new;
        true
    }
    pub fn scroll(&mut self, dx: i32, dy: i32) {
        self.x = self.x.saturating_add_signed(dx);
        self.y = self.y.saturating_add_signed(dy);
    }
    pub fn set_pages(&mut self, pages: u32) {
        self.pages = pages.max(1);
        self.page = self.page.min(self.pages - 1);
    }
}

pub fn raster_width(view: &View, viewport: Viewport) -> u32 {
    ((viewport.width.max(1) as f32 * view.zoom).round() as u32).clamp(100, 3400)
}

/// Crop a pre-rasterized page, clamping scrolling to the actual image dimensions.
pub fn crop_bounds(image_width: u32, image_height: u32, viewport: Viewport, x: u32, y: u32) -> (u32,u32,u32,u32) {
    let w = image_width.min(viewport.width.max(1));
    let h = image_height.min(viewport.height.max(1));
    let x = x.min(image_width.saturating_sub(w));
    let y = y.min(image_height.saturating_sub(h));
    (x,y,w,h)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn navigation_is_bounded() {
        let mut v = View::new(3);
        assert!(!v.step_page(-1));
        assert!(v.step_page(1));
        assert!(v.step_page(1));
        assert!(!v.step_page(1));
        assert!(v.go_to(1));
        assert_eq!(v.page, 0);
    }
    #[test] fn scroll_never_underflows() {
        let mut v = View::new(1);
        v.scroll(-100, -10);
        assert_eq!((v.x,v.y), (0,0));
        assert_eq!(crop_bounds(1000,1500,Viewport { width: 500,height: 700 },u32::MAX,u32::MAX), (500,800,500,700));
    }
    #[test] fn zoom_is_bounded() {
        let mut v = View::new(1);
        for _ in 0..30 { v.adjust_zoom(0.25); }
        assert_eq!(v.zoom, 3.0);
        for _ in 0..30 { v.adjust_zoom(-0.25); }
        assert_eq!(v.zoom, 0.5);
    }
}
