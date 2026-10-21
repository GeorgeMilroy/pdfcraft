//! The UI profile in the Tools panel, Organize pages, the Pages and Bookmarks panels and page
//! clipboard: what the profile refuses is not drawn, and the default profile shows it all.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::features::{Feature, FeatureSet};
use pdfcraft_ui_egui::profile::UiProfile;
use pdfcraft_ui_egui::{LeftPanel, PdfCraftApp, RightPanel};

fn fixture(n: usize) -> Vec<u8> {
    let mut objs: Vec<String> = vec!["<< /Type /Catalog /Pages 2 0 R >>".into()];
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 3 + i)).collect();
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {n} /MediaBox [0 0 200 300] >>", kids.join(" ")));
    for _ in 0..n {
        objs.push("<< /Type /Page /Parent 2 0 R >>".into());
    }
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}

fn harness(profile: UiProfile) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("doc.pdf", None, fixture(2)).unwrap();
        app.profile = profile;
        app
    });
    h.run_steps(4);
    h.state_mut().set_option("mode", "all").unwrap();
    h.run_steps(2);
    h
}

/// Only the comment features: a typical "review" embedding.
fn comments() -> UiProfile {
    let set: FeatureSet = [Feature::Comments, Feature::Markup, Feature::Shapes, Feature::Ink, Feature::Stamps].into_iter().collect();
    UiProfile { features: Some(set), ..UiProfile::default() }
}

/// Every feature but `off`.
fn all_but(off: Feature) -> UiProfile {
    let mut set = FeatureSet::all();
    set.remove(off);
    UiProfile { features: Some(set), ..UiProfile::default() }
}

fn pages(h: &Harness<'_, PdfCraftApp>) -> usize {
    let (_, id) = h.state().active_ids().unwrap();
    h.state().session.get(id).unwrap().info.pages.len()
}

fn shows(h: &Harness<'_, PdfCraftApp>, label: &str) -> bool {
    h.query_all_by_label(label).count() > 0
}

fn right_click(h: &mut Harness<'static, PdfCraftApp>, label: &str) {
    let at = h.get_by_label(label).rect().center();
    h.hover_at(at);
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed: true, modifiers: Default::default() });
    h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed: false, modifiers: Default::default() });
    h.run_steps(2);
}

#[test]
fn all_tools_lists_only_the_groups_whose_feature_is_on() {
    for (profile, shown) in [(UiProfile::default(), true), (comments(), false), (UiProfile { comments_only: true, ..UiProfile::default() }, false)] {
        let mut h = harness(profile);
        h.state_mut().all_tools_expanded = true;
        h.run_steps(2);
        assert!(shows(&h, "Add comments"));
        for group in ["Organize pages", "Edit a PDF", "Redact a PDF"] {
            assert_eq!(shows(&h, group), shown, "{group}");
        }
    }
}

#[test]
fn a_refused_tool_group_does_not_open() {
    let mut h = harness(comments());
    h.state_mut().left = LeftPanel::Tool("redact");
    h.run_steps(2);
    assert_eq!(h.state().left, LeftPanel::AllTools);
    assert!(!shows(&h, "Redact all"));
}

#[test]
fn the_redact_footer_shows_only_what_the_profile_allows() {
    let mut hidden = UiProfile::default();
    hidden.hidden.insert("redact.apply".into());
    for (profile, redact_all) in [(UiProfile::default(), true), (hidden, false)] {
        let mut h = harness(profile);
        h.state_mut().left = LeftPanel::Tool("redact");
        h.run_steps(2);
        assert!(shows(&h, "Clear all"));
        assert_eq!(shows(&h, "Redact all"), redact_all);
    }
}

#[test]
fn a_menu_entry_leaves_out_the_actions_the_profile_refuses() {
    let mut hidden = UiProfile::default();
    hidden.hidden.insert("edit.header_footer.remove".into());
    for (profile, remove) in [(UiProfile::default(), true), (hidden, false)] {
        let mut h = harness(profile);
        h.state_mut().left = LeftPanel::Tool("edit");
        h.run_steps(2);
        h.get_by_label("Header and footer").click();
        h.run_steps(2);
        assert!(shows(&h, "Add header & footer…"));
        assert_eq!(shows(&h, "Remove header & footer"), remove);
    }
}

#[test]
fn organize_pages_shows_only_with_the_pages_feature_and_hides_refused_buttons() {
    let organize = |profile: UiProfile| {
        let mut h = harness(profile);
        h.state_mut().views[0].organize = true;
        h.run_steps(3);
        h
    };
    let h = organize(UiProfile::default());
    assert!(shows(&h, "Rotate clockwise") && shows(&h, "Extract pages to a new document") && shows(&h, "Split"));
    let h = organize(comments());
    assert!(!shows(&h, "Rotate clockwise"), "no Organize panel without the pages feature");
    let h = organize(all_but(Feature::Export));
    assert!(shows(&h, "Rotate clockwise"));
    assert!(!shows(&h, "Extract pages to a new document") && !shows(&h, "Split"), "extracting and splitting are exports");
}

#[test]
fn queued_page_actions_the_profile_refuses_do_not_run() {
    for (profile, opens) in [(UiProfile::default(), true), (all_but(Feature::Export), false)] {
        let mut h = harness(profile);
        h.state_mut().views[0].pending_action = Some(pdfcraft_ui_egui::canvas::ViewAction::Extract);
        h.run_steps(2);
        assert_eq!(h.state().dialog.is_some(), opens);
    }
}

#[test]
fn the_pages_panel_menu_leaves_out_what_the_profile_refuses() {
    for (profile, shown) in [(UiProfile::default(), true), (comments(), false)] {
        let mut h = harness(profile);
        h.state_mut().choose_right_panel(Some(RightPanel::Pages));
        h.run_steps(3);
        right_click(&mut h, "Page 1");
        for item in ["Delete pages", "Cut", "Paste after", "Extract pages", "Replace pages…"] {
            assert_eq!(shows(&h, item), shown, "{item}");
        }
    }
}

#[test]
fn a_refused_cut_leaves_the_clipboard_and_the_pages() {
    let mut h = harness(UiProfile::default());
    h.state_mut().copy_pages(true);
    assert!(h.state().page_clipboard.is_some());
    assert_eq!(pages(&h), 1);

    let mut h = harness(comments());
    h.state_mut().copy_pages(true);
    assert!(h.state().page_clipboard.is_none(), "nothing cut, nothing on the clipboard");
    assert_eq!(pages(&h), 2);
}

#[test]
fn bookmark_editing_needs_the_bookmarks_feature() {
    for (profile, shown) in [(UiProfile::default(), true), (comments(), false)] {
        let mut h = harness(profile);
        h.state_mut().choose_right_panel(Some(RightPanel::Bookmarks));
        h.run_steps(3);
        assert_eq!(h.query_all_by_label_contains("New bookmark").count() > 0, shown);
    }
}

#[test]
fn the_pages_panel_menu_does_not_open_when_it_has_nothing_to_offer() {
    for (profile, opens) in [(UiProfile::default(), true), (comments(), false)] {
        let mut h = harness(profile);
        h.state_mut().choose_right_panel(Some(RightPanel::Pages));
        h.run_steps(3);
        right_click(&mut h, "Page 2");
        // Opening the menu makes the page under it the selection.
        assert_eq!(h.state().views[0].selected.contains(&1), opens, "the selection changed although no menu opened");
    }
}

#[test]
fn the_redact_footer_goes_when_both_its_buttons_are_refused_at_runtime() {
    let mut h = harness(UiProfile::default());
    h.state_mut().left = LeftPanel::Tool("redact");
    h.run_steps(2);
    assert!(shows(&h, "Clear all") && shows(&h, "Redact all"));
    let mut both = UiProfile::default();
    both.hidden.extend(["redact.apply".to_string(), "redact.clear".to_string()]);
    h.state_mut().set_profile(both);
    h.run_steps(2);
    assert_eq!(h.state().left, LeftPanel::Tool("redact"), "marking is still allowed");
    assert!(!shows(&h, "Clear all") && !shows(&h, "Redact all"));
}
