//! A brand's About link opens through `PdfCraftApp::open_url` (a new `noopener` tab on the web,
//! the system browser on the desktop), not through egui's hyperlink, which on the web opens a
//! plain click in the viewer's own frame. Its own test binary: the brand is process-wide.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;
use pdfcraft_ui_egui::brand::{self, Brand};

const URL: &str = "https://acme.example/about";

#[test]
fn the_about_link_opens_through_open_url() {
    brand::set(Brand { app_name: "Acme PDF".into(), about_url: Some(URL.into()), storage_key: "acme-about".into(), ..Brand::default() });
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app
    });
    h.run_steps(2);
    assert!(h.state_mut().execute("help.about"));
    h.run_steps(4);
    h.get_by_label(URL).click();
    h.run_steps(3);
    assert_eq!(h.state().last_opened_url.as_deref(), Some(URL));
}
