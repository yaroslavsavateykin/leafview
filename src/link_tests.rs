use crate::links::*;
use ratatui::layout::{Rect, Size};
use std::{path::Path, sync::Arc};
fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn index() -> LinkIndex {
    LinkIndex::extract(&fixture("links.pdf")).unwrap()
}
#[test]
fn annotations_and_destinations() {
    let index = index();
    assert_eq!(index.pages.len(), 5);
    let p = index.pages[0].as_ref().unwrap();
    assert_eq!(p.links.len(), 8);
    assert!(
        matches!(&p.links[0].action,LinkAction::External(url) if url=="https://example.org/a?x=1&y=2")
    );
    assert!(matches!(
        &p.links[1].action,
        LinkAction::Internal(Destination {
            page: 1,
            kind: DestinationKind::Xyz {
                left: Some(110.),
                top: Some(700.),
                zoom: Some(1.25)
            },
            ..
        })
    ));
    assert!(matches!(
        &p.links[2].action,
        LinkAction::Internal(Destination {
            page: 1,
            kind: DestinationKind::FitH(Some(650.)),
            ..
        })
    ));
    assert!(matches!(
        &p.links[3].action,
        LinkAction::Internal(Destination {
            page: 1,
            kind: DestinationKind::Fit,
            ..
        })
    ));
    for i in 4..7 {
        assert!(matches!(p.links[i].action, LinkAction::Unsupported(_)));
    }
    assert!(index.pages[1].as_ref().unwrap().links.is_empty());
    let p = index.pages[2].as_ref().unwrap();
    assert_eq!(p.space.rotation, 90);
    assert_eq!(p.space.crop.x0, 110.);
}
#[test]
fn typst_annotations() {
    let index = LinkIndex::extract(&fixture("typst-links.pdf")).unwrap();
    let page = index.pages[0].as_ref().unwrap();
    assert!(
        matches!(&page.links[0].action,LinkAction::External(url) if url.starts_with("https://doi.org/"))
    );
    assert!(matches!(
        page.links[1].action,
        LinkAction::Internal(Destination { page: 1, .. })
    ));
}
#[test]
fn coordinate_matrix_zoom_scroll_rotation_center_and_cells() {
    let index = index();
    for page in [0, 2, 3, 4] {
        let links = index.pages[page].clone().unwrap();
        let r = links.links[0].bounds;
        let pdf_center = ((r.x0 + r.x1) / 2., (r.y0 + r.y1) / 2.);
        for zoom in [0.5, 1., 1.25, 2., 3.] {
            for font in [(5, 10), (8, 16), (12, 24)] {
                let (w, h) = links.space.size();
                let raster = ((w * zoom) as u32, (h * zoom) as u32);
                let (u, v) = links.space.to_display(pdf_center.0, pdf_center.1);
                let (px, py) = (u * raster.0 as f64, v * raster.1 as f64);
                let (back_x, back_y) = links.space.to_pdf(u, v);
                assert!(
                    (back_x - pdf_center.0).abs() < 0.001 && (back_y - pdf_center.1).abs() < 0.001
                );
                for scroll in [
                    (0, 0),
                    ((px as u32).saturating_sub(30), 0),
                    (0, (py as u32).saturating_sub(30)),
                    (
                        (px as u32).saturating_sub(30),
                        (py as u32).saturating_sub(30),
                    ),
                ] {
                    let crop = (scroll.0, scroll.1, raster.0 - scroll.0, raster.1 - scroll.1);
                    let g = RenderGeometry {
                        document_id: 1,
                        revision: 2,
                        page: page as u32,
                        generation: 7,
                        raster,
                        crop,
                        links: Some(links.clone()),
                    };
                    let size = Size::new(
                        crop.2.div_ceil(font.0) as u16,
                        crop.3.div_ceil(font.1) as u16,
                    );
                    let area = image_area(Rect::new(4, 3, size.width + 20, size.height + 10), size);
                    let col = area.x + ((px - scroll.0 as f64) / font.0 as f64) as u16;
                    let row = area.y + ((py - scroll.1 as f64) / font.1 as f64) as u16;
                    assert_eq!(
                        g.hit(col, row, area, font),
                        Some(0),
                        "page {page}, zoom {zoom}, font {font:?}, scroll {scroll:?}"
                    );
                    assert_eq!(g.hit(area.x - 1, row, area, font), None);
                    assert_eq!(g.hit(col, area.bottom(), area, font), None);
                    assert!(g.is_current(2, page as u32, 7));
                    assert!(!g.is_current(3, page as u32, 7));
                    assert!(!g.is_current(2, page as u32, 8));
                }
            }
        }
    }
}
#[test]
fn y_axis_and_ambiguous_small_links() {
    let index = index();
    let p = index.pages[0].clone().unwrap();
    let mut g = RenderGeometry {
        document_id: 1,
        revision: 1,
        page: 0,
        generation: 1,
        raster: (600, 800),
        crop: (0, 0, 600, 800),
        links: Some(p.clone()),
    };
    let area = Rect::new(0, 0, 60, 40);
    assert_eq!(g.hit(10, 4, area, (10, 20)), Some(0)); // PDF y ~730, not 110!
    assert_eq!(g.hit(40, 4, area, (10, 20)), None);
    assert_eq!(g.hit(6, 37, area, (10, 20)), None); // tiny rectangle
    let mut overlapping = (*p).clone();
    overlapping.links.push(overlapping.links[0].clone());
    g.links = Some(Arc::new(overlapping));
    assert_eq!(g.hit(10, 4, area, (10, 20)), None);
}
#[test]
fn destination_positions_and_fit() {
    let space = index().pages[0].as_ref().unwrap().space;
    let mut d = Destination {
        page: 0,
        space,
        kind: DestinationKind::Xyz {
            left: Some(110.),
            top: Some(700.),
            zoom: None,
        },
    };
    assert_eq!(d.offsets((600, 800), (0, 0)), (100, 120));
    assert_eq!(d.view_zoom((600, 400), 1.25), 1.25);
    d.kind = DestinationKind::Fit;
    assert_eq!(d.view_zoom((600, 400), 1.), 0.5);
    d.kind = DestinationKind::FitH(Some(700.));
    assert_eq!(d.offsets((600, 800), (0, 0)), (0, 120));
    d.kind = DestinationKind::FitV(Some(110.));
    assert_eq!(d.offsets((600, 800), (0, 0)), (100, 0));
    d.kind = DestinationKind::FitR(PdfRect {
        x0: 110.,
        y0: 120.,
        x1: 310.,
        y1: 320.,
    });
    assert_eq!(d.view_zoom((600, 400), 1.), 2.);
    assert_eq!(d.offsets((600, 800), (0, 0)), (100, 500));
    d.space.rotation = 90;
    d.kind = DestinationKind::FitH(Some(700.));
    assert_eq!(d.offsets((800, 600), (0, 0)), (680, 0));
    d.kind = DestinationKind::FitV(Some(110.));
    assert_eq!(d.offsets((800, 600), (0, 0)), (800, 100));
}
#[test]
fn uri_and_terminal_injection() {
    for url in [
        "javascript:alert(1)",
        "file:///tmp/x",
        "https://",
        "https://x\nsh",
        "https://a\u{1b}[2J",
        "mailto:a@b",
        "https://a@evil",
    ] {
        assert!(validate_url(url).is_err(), "{url}");
    }
    let url = "https://example.org/?a=$(touch);b='quoted'&c=42";
    let command = browser_command(url).unwrap();
    assert_eq!(command.get_program(), "xdg-open");
    assert_eq!(command.get_args().collect::<Vec<_>>(), vec![url]);
    assert_eq!(sanitize("hello\x1b\nworld\u{202e}"), "helloworld");
}
