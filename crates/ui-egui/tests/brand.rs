//! A custom brand replaces PdfCraft's name, ArtCraft's marks and the community links
//! everywhere the interface shows them (crate::brand). Its own test binary: the brand is
//! process-wide.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;
use pdfcraft_ui_egui::brand::{self, Brand};

fn acme() -> Brand {
    Brand {
        app_name: "Acme PDF".into(),
        version: Some("9.9.9".into()),
        logo_light: None,
        logo_dark: None,
        mark: None,
        accent: Some((egui::Color32::from_rgb(0x2B, 0x6C, 0xB0), egui::Color32::from_rgb(0x63, 0xA4, 0xF0))),
        about_text: Some("Acme's viewer. Based on PdfCraft.".into()),
        about_url: None,
        community_links: false,
        credits: false,
        storage_key: "acme".into(),
    }
}

fn no_trademarks(h: &Harness<'_, PdfCraftApp>, place: &str) {
    for mark in ["ArtCraft", "Discord", "Join our Discord", "PdfCraft web page", "PdfCraft on GitHub"] {
        assert!(h.query_by_label_contains(mark).is_none(), "{place}: {mark:?} is shown");
    }
}

#[test]
fn a_custom_brand_names_the_app_everywhere_and_drops_the_marks() {
    brand::set(acme());
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app
    });
    h.run_steps(4);
    // Home: the welcome line, no community card, no Discord button in the tab strip.
    assert!(h.query_by_label("Welcome to Acme PDF").is_some(), "home greets with the brand name");
    assert!(h.query_by_label_contains("Welcome to PdfCraft").is_none());
    no_trademarks(&h, "home");
    assert_eq!(h.state().window_title, "Acme PDF");
    // The community links don't run either (palette, shortcuts, set_option).
    for l in pdfcraft_engine::links::LINKS {
        assert!(!h.state_mut().execute(l.command), "{} ran", l.command);
    }
    assert_eq!(h.state().last_opened_url, None);

    // About: name, version and text from the brand; no "Part of ArtCraft".
    h.state_mut().set_option("dialog", "about").unwrap();
    h.run_steps(4);
    assert!(h.query_by_label("Acme PDF").is_some(), "About names the brand");
    assert!(h.query_by_label("Version 9.9.9").is_some());
    assert!(h.query_by_label("Acme's viewer. Based on PdfCraft.").is_some(), "the brand's About text, kept as written");
    assert!(h.query_by_label("Part of").is_none());
    // Without the credits, About has no Contributors or Models tab.
    for tab in ["Contributors", "Models"] {
        assert!(h.query_by_label(tab).is_none(), "About shows the {tab} tab");
    }
    no_trademarks(&h, "about");

    // Translated strings are renamed too.
    h.state_mut().set_option("language", "de").unwrap();
    h.state_mut().set_option("dialog", "none").unwrap();
    h.run_steps(4);
    assert!(h.query_by_label_contains("PdfCraft").is_none(), "German home names no PdfCraft");
}
