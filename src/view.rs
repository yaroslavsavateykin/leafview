//! UI-independent navigation and geometry (unit testable without a terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    Page,
    Width,
    Manual,
}
#[derive(Debug, Clone)]
pub struct View {
    pub page: u32,
    pub pages: u32,
    pub zoom: f32,
    pub fit: Fit,
    /// Page-relative scroll in rendered pixels.
    pub x: u32,
    pub y: u32,
}

impl View {
    pub fn new(pages: u32) -> Self {
        Self {
            page: 0,
            pages: pages.max(1),
            zoom: 1.0,
            fit: Fit::Page,
            x: 0,
            y: 0,
        }
    }
    pub fn step_page(&mut self, delta: i32) -> bool {
        let next = (self.page as i64 + delta as i64).clamp(0, self.pages as i64 - 1) as u32;
        if next == self.page {
            return false;
        }
        self.page = next;
        // Page turns preserve zoom and pan; the renderer clamps to page bounds.
        true
    }
    pub fn go_to(&mut self, page_1_based: u32) -> bool {
        let next = page_1_based.saturating_sub(1).min(self.pages - 1);
        if next == self.page {
            return false;
        }
        self.page = next;
        true
    }
    /// Fit uses physical pixel dimensions (not terminal cell aspect).
    pub fn apply_fit(&mut self, viewport: Viewport, aspect: f64) -> bool {
        if self.fit == Fit::Manual || !aspect.is_finite() || aspect <= 0. {
            return false;
        }
        let zoom = if self.fit == Fit::Width {
            1.
        } else {
            (viewport.height as f64 / (viewport.width.max(1) as f64 * aspect)).min(1.) as f32
        }
        .max(0.01);
        let changed = (self.zoom - zoom).abs() > 0.0001;
        if changed {
            self.x = 0;
            self.y = 0;
        }
        self.zoom = zoom;
        changed
    }
    pub fn adjust_zoom(&mut self, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        self.fit = Fit::Manual;
        let new = (self.zoom + delta).clamp(0.05, 3.0);
        if (new - self.zoom).abs() < 0.001 {
            return false;
        }
        self.x = (self.x as f32 * new / self.zoom) as u32;
        self.y = (self.y as f32 * new / self.zoom) as u32;
        self.zoom = new;
        true
    }
    /// Keep the page point under the pointer stationary, including centered
    /// pages and the raster-width cap. Anchor is viewport-relative in pixels.
    pub fn zoom_at(
        &mut self,
        delta: f32,
        viewport: Viewport,
        aspect: f64,
        anchor: (f64, f64),
    ) -> bool {
        let old_width = raster_width(self, viewport) as f64;
        let old = (self.x, self.y);
        // Avoid accumulating floating-point drift over many wheel ticks.
        let target = ((self.zoom + delta) * 100.).round() / 100.;
        if !self.adjust_zoom(target - self.zoom) {
            return false;
        }
        let new_width = raster_width(self, viewport) as f64;
        let ratio = new_width / old_width;
        let offset = |scroll: u32, pointer: f64, extent: f64, dimension: f64| {
            let old_padding = (extent - dimension).max(0.) / 2.;
            let new_padding = (extent - dimension * ratio).max(0.) / 2.;
            let point = (scroll as f64 + pointer - old_padding).clamp(0., dimension);
            (point * ratio - pointer + new_padding)
                .max(0.)
                .min((dimension * ratio - extent).max(0.))
                .round() as u32
        };
        self.x = offset(old.0, anchor.0, viewport.width as f64, old_width);
        self.y = offset(old.1, anchor.1, viewport.height as f64, old_width * aspect);
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
    ((viewport.width.max(1) as f32 * view.zoom).round() as u32).clamp(1, 3400)
}

/// Crop a pre-rasterized page, clamping scrolling to the actual image dimensions.
pub fn crop_bounds(
    image_width: u32,
    image_height: u32,
    viewport: Viewport,
    x: u32,
    y: u32,
) -> (u32, u32, u32, u32) {
    let w = image_width.min(viewport.width.max(1));
    let h = image_height.min(viewport.height.max(1));
    let x = x.min(image_width.saturating_sub(w));
    let y = y.min(image_height.saturating_sub(h));
    (x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opening_fits_portrait_landscape_and_resize() {
        for (viewport, aspect) in [
            (
                Viewport {
                    width: 1200,
                    height: 600,
                },
                297. / 210.,
            ),
            (
                Viewport {
                    width: 600,
                    height: 1000,
                },
                0.6,
            ),
            (
                Viewport {
                    width: 1800,
                    height: 300,
                },
                1.5,
            ),
        ] {
            let mut view = View::new(1);
            view.apply_fit(viewport, aspect);
            let width = raster_width(&view, viewport);
            assert!(width <= viewport.width);
            assert!(width as f64 * aspect <= viewport.height as f64 + 1.5);
            view.adjust_zoom(0.1);
            assert_eq!(view.fit, Fit::Manual);
            let zoom = view.zoom;
            view.apply_fit(
                Viewport {
                    width: 300,
                    height: 200,
                },
                aspect,
            );
            assert_eq!(view.zoom, zoom);
            view.fit = Fit::Width;
            view.apply_fit(viewport, aspect);
            assert_eq!(view.zoom, 1.);
        }
    }
    #[test]
    fn navigation_is_bounded() {
        let mut v = View::new(3);
        assert!(!v.step_page(-1));
        assert!(v.step_page(1));
        assert!(v.step_page(1));
        assert!(!v.step_page(1));
        assert!(v.go_to(1));
        assert_eq!(v.page, 0);
    }
    #[test]
    fn page_navigation_preserves_zoom_and_pan() {
        let mut view = View::new(5);
        view.adjust_zoom(1.0);
        view.scroll(320, 640);
        for delta in [1, 1, -1, -1, -1, 4, 1] {
            view.step_page(delta);
            assert_eq!((view.zoom, view.x, view.y), (2.0, 320, 640));
        }
        for page in [1, 3, 5, 999, 0] {
            view.go_to(page);
            assert_eq!((view.zoom, view.x, view.y), (2.0, 320, 640));
        }
        assert_eq!(
            crop_bounds(
                400,
                500,
                Viewport {
                    width: 300,
                    height: 400
                },
                view.x,
                view.y
            ),
            (100, 100, 300, 400)
        );
    }
    #[test]
    fn scroll_never_underflows() {
        let mut v = View::new(1);
        v.scroll(-100, -10);
        assert_eq!((v.x, v.y), (0, 0));
        assert_eq!(
            crop_bounds(
                1000,
                1500,
                Viewport {
                    width: 500,
                    height: 700
                },
                u32::MAX,
                u32::MAX
            ),
            (500, 800, 500, 700)
        );
    }
    #[test]
    fn stress_and_invalid_zoom() {
        let mut v = View::new(120);
        assert!(!v.adjust_zoom(f32::NAN));
        assert!(!v.adjust_zoom(f32::INFINITY));
        for i in 0..1000 {
            v.go_to(i);
            v.adjust_zoom(if i % 2 == 0 { 0.25 } else { -0.25 });
            v.scroll(i32::MAX, i32::MAX);
            let viewport = Viewport {
                width: i % 300,
                height: i % 100,
            };
            let (x, y, w, h) = crop_bounds(500, 700, viewport, v.x, v.y);
            assert!(x + w <= 500 && y + h <= 700);
            assert!(raster_width(&v, viewport) <= 3400);
        }
        v.set_pages(2);
        assert_eq!(v.page, 1);
        assert!(v.zoom.is_finite());
        assert_eq!(
            crop_bounds(
                0,
                0,
                Viewport {
                    width: 0,
                    height: 0
                },
                0,
                0
            ),
            (0, 0, 0, 0)
        );
    }
    #[test]
    fn pointer_zoom_preserves_page_point() {
        let viewport = Viewport {
            width: 800,
            height: 600,
        };
        let mut v = View::new(2);
        v.zoom = 2.;
        v.x = 300;
        v.y = 500;
        let anchor = (200., 250.);
        let before = (
            (v.x as f64 + anchor.0) / 1600.,
            (v.y as f64 + anchor.1) / 2400.,
        );
        assert!(v.zoom_at(0.05, viewport, 1.5, anchor));
        let width = raster_width(&v, viewport) as f64;
        assert!(((v.x as f64 + anchor.0) / width - before.0).abs() < 0.001);
        assert!(((v.y as f64 + anchor.1) / (width * 1.5) - before.1).abs() < 0.001);
        let mut centered = View::new(1);
        centered.zoom = 0.5;
        assert!(centered.zoom_at(0.5, viewport, 1.5, (400., 300.)));
        assert_eq!(centered.x, 0);
        assert_eq!(centered.y, 300);
        v.zoom = 3.;
        assert!(!v.zoom_at(0.05, viewport, 1.5, anchor));
    }
    #[test]
    fn zoom_is_bounded() {
        let mut v = View::new(1);
        for _ in 0..30 {
            v.adjust_zoom(0.25);
        }
        assert_eq!(v.zoom, 3.0);
        for _ in 0..30 {
            v.adjust_zoom(-0.25);
        }
        assert_eq!(v.zoom, 0.05);
    }
}
