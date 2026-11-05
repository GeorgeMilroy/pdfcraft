//! The UI profile on what pdfcraft 0.16 to 0.18 added: the hotbar, Home's pinned tools and the
//! card-based Preferences. What the profile refuses is hidden; the default profile changes
//! nothing.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;
use pdfcraft_ui_egui::features::Feature;
use pdfcraft_ui_egui::profile::UiProfile;

/// See `tests/view.rs`: WARP's shader JIT crashes when two devices compile at once on ARM64.
static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gpu() -> std::sync::MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn fixture() -> Vec<u8> {
    let objs =
        ["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 300] >>", "<< /Type /Page /Parent 2 0 R >>"];
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

/// The app under `profile`, with a one-page document open when `document`, else on Home; `setup`
/// runs after the profile is set.
fn harness(profile: UiProfile, document: bool, setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        if document {
            app.open_bytes("doc.pdf", None, fixture()).unwrap();
        }
        app.set_profile(profile);
        setup(&mut app);
        app
    });
    h.run_steps(6);
    h
}

/// Every feature but `off`.
fn without(off: &[Feature]) -> UiProfile {
    UiProfile { features: Some(Feature::ALL.into_iter().filter(|f| !off.contains(f)).collect()), ..UiProfile::default() }
}

fn hiding(ids: &[&str]) -> UiProfile {
    UiProfile { hidden: ids.iter().map(|s| s.to_string()).collect(), ..UiProfile::default() }
}

#[test]
fn the_hotbar_leaves_out_fill_and_sign_without_its_feature() {
    let _gpu = gpu();
    let shown = harness(UiProfile::default(), true, |_| {});
    assert!(shown.query_all_by_label_contains("Fill & Sign:").count() >= 1, "the default hotbar has Fill & Sign");
    let hidden = harness(without(&[Feature::FillSign]), true, |_| {});
    assert_eq!(hidden.query_all_by_label_contains("Fill & Sign:").count(), 0, "no Fill & Sign button without fillSign");
}

#[test]
fn preferences_show_only_the_cards_of_allowed_features() {
    let _gpu = gpu();
    let open = |app: &mut PdfCraftApp| app.set_option("dialog", "preferences").unwrap();
    let cards = ["Name on new comments", "Flatten Fill & Sign when saving", "Enable Acrobat JavaScript"];
    let all = harness(UiProfile::default(), true, open);
    for label in cards {
        assert!(all.query_by_label(label).is_some(), "{label} is in the default Preferences");
    }
    let comments = [Feature::Comments, Feature::Markup, Feature::Shapes, Feature::Ink, Feature::Stamps, Feature::Measurement];
    let mut off = comments.to_vec();
    off.extend([Feature::FillSign, Feature::JavaScript]);
    let none = harness(without(&off), true, open);
    assert!(none.query_all_by_label("Preferences").count() >= 1, "Preferences itself still opens");
    for label in cards {
        assert!(none.query_by_label(label).is_none(), "{label} shows without its feature");
    }
}

#[test]
fn home_offers_pinning_only_with_preferences_and_a_group_to_pin() {
    let _gpu = gpu();
    let home = harness(UiProfile::default(), false, |_| {});
    assert!(home.query_by_label("Pin a tool…").is_some(), "the default Home offers pinning");
    let no_prefs = harness(hiding(&["app.preferences"]), false, |_| {});
    assert!(no_prefs.query_by_label("Pin a tool…").is_none(), "pinning is a preference");
    let groups: Vec<String> = pdfcraft_engine::catalog::TOOL_GROUPS.iter().map(|g| format!("group:{}", g.id)).collect();
    let no_groups = harness(UiProfile { hidden: groups.into_iter().collect(), ..UiProfile::default() }, false, |_| {});
    assert!(no_groups.query_by_label("Pinned Tools").is_none(), "nothing to pin, no Pinned Tools");
}

#[test]
fn hotbar_settings_are_a_preference() {
    let _gpu = gpu();
    let shown = harness(UiProfile::default(), true, |_| {});
    assert_eq!(shown.query_all_by_label("Hotbar settings").count(), 1, "the default hotbar has its settings");
    let hidden = harness(hiding(&["app.preferences"]), true, |_| {});
    assert_eq!(hidden.query_all_by_label("Hotbar settings").count(), 0, "no hotbar settings without preferences");
}

#[test]
fn the_tools_panel_pins_to_home_only_where_home_shows_and_preferences_are_allowed() {
    let _gpu = gpu();
    let pinned = |app: &mut PdfCraftApp| {
        assert!(app.pin_tool("comment"));
    };
    // A pinned group's row shows its Unpin from Home toggle.
    let shown = harness(UiProfile::default(), true, pinned);
    assert!(shown.query_all_by_label("Unpin from Home").count() >= 1, "the default Tools panel offers unpinning");
    let no_home = harness(hiding(&["ui:home"]), true, pinned);
    assert_eq!(no_home.query_all_by_label("Unpin from Home").count(), 0, "no pinning to a Home that doesn't show");
    let no_prefs = harness(hiding(&["app.preferences"]), true, pinned);
    assert_eq!(no_prefs.query_all_by_label("Unpin from Home").count(), 0, "pinning is a preference");
}

#[test]
fn a_refused_fill_tool_request_changes_nothing() {
    let _gpu = gpu();
    let mut h = harness(hiding(&["sign.fill.date"]), true, |_| {});
    let before = h.state().fill_tool;
    assert!(h.state_mut().set_option("quick", "fill-date").is_err());
    assert_eq!(h.state().fill_tool, before, "the last Fill & Sign tool is kept");
}
