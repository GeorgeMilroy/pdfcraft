//! The UI profile on what pdfcraft 0.19 to 0.21 added: JavaScript links, page actions, Ctrl+M
//! for the Comments panel and a picture for the exact image field clicked. What the profile
//! refuses is refused however it is asked for; the default profile changes nothing.

use egui::os::OperatingSystem;
use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::features::{Feature, FeatureSet};
use pdfcraft_ui_egui::profile::UiProfile;
use pdfcraft_ui_egui::{PdfCraftApp, RightPanel};

/// A one-page PDF with a text field `log`: `annots` come first on its page (a widget among them
/// is a field too), and `page_keys` go in the page's dictionary.
fn pdf(page_keys: &str, annots: &[&str]) -> Vec<u8> {
    let mut objs: Vec<String> = vec![
        String::new(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 300] >>".into(),
        String::new(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (log) /V () /Rect [20 250 180 270] /P 3 0 R /F 4 >>".into(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into(),
    ];
    let mut refs: Vec<String> = Vec::new();
    let mut fields = vec!["4 0 R".to_string()];
    for a in annots {
        objs.push((*a).to_string());
        refs.push(format!("{} 0 R", objs.len()));
        if a.contains("/Subtype /Widget") {
            fields.push(format!("{} 0 R", objs.len()));
        }
    }
    refs.push("4 0 R".into());
    objs[0] = format!(
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{}] /DA (/Helv 0 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> >>",
        fields.join(" ")
    );
    objs[2] = format!("<< /Type /Page /Parent 2 0 R /Annots [{}] {page_keys} >>", refs.join(" "));
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

/// A link whose action is JavaScript writing "clicked" into `log`.
const SCRIPT_LINK: &str = "<< /Type /Annot /Subtype /Link /Rect [20 20 180 60] /Border [0 0 0] /A << /S /JavaScript /JS (this.getField\\('log'\\).value = 'clicked';) >> >>";

/// The value of field `name` in the active document.
fn field(app: &PdfCraftApp, name: &str) -> String {
    let (_, id) = app.active_ids().unwrap();
    app.session.get(id).unwrap().form.iter().find(|f| f.name == name).map(|f| f.value.join("")).unwrap_or_default()
}

fn toast(app: &PdfCraftApp) -> Option<&str> {
    app.toast.as_ref().map(|(t, _)| t.as_str())
}

fn hiding(ids: &[&str]) -> UiProfile {
    UiProfile { hidden: ids.iter().map(|s| s.to_string()).collect(), ..UiProfile::default() }
}

/// Every feature but `off`.
fn without(off: &[Feature]) -> UiProfile {
    UiProfile { features: Some(Feature::ALL.into_iter().filter(|f| !off.contains(f)).collect()), ..UiProfile::default() }
}

/// See `tests/view.rs`: WARP's shader JIT crashes when two devices compile at once on ARM64.
static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gpu() -> std::sync::MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The app with `pdf` opened under `profile`, after a few frames.
fn harness(profile: UiProfile, pdf: Vec<u8>) -> egui_kittest::Harness<'static, PdfCraftApp> {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.set_profile(profile);
        app.open_bytes("doc.pdf", None, pdf).unwrap();
        app
    });
    h.run_steps(6);
    h
}

/// Frames until `done` holds (a script may finish on a worker), or about two seconds.
fn settle(h: &mut egui_kittest::Harness<'static, PdfCraftApp>, done: impl Fn(&PdfCraftApp) -> bool) {
    for _ in 0..200 {
        if done(h.state()) {
            return;
        }
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn a_pages_open_actions_run_only_with_javascript() {
    let _gpu = gpu();
    let opened = pdf("/AA << /O << /S /JavaScript /JS (this.getField\\('log'\\).value = 'opened';) >> >>", &[]);
    let mut h = harness(UiProfile::default(), opened.clone());
    settle(&mut h, |app| field(app, "log") == "opened");
    assert_eq!(field(h.state(), "log"), "opened", "the default profile runs them");
    let mut h = harness(without(&[Feature::JavaScript]), opened);
    settle(&mut h, |app| field(app, "log") == "opened");
    assert!(!h.state().session.javascript(), "the profile keeps the engine's JavaScript off");
    assert_eq!(field(h.state(), "log"), "", "nothing ran");
}

#[test]
fn a_javascript_link_runs_only_with_javascript() {
    let _gpu = gpu();
    for (profile, runs) in [(UiProfile::default(), true), (without(&[Feature::JavaScript]), false)] {
        let mut h = harness(profile, pdf("", &[SCRIPT_LINK]));
        let (_, id) = h.state().active_ids().unwrap();
        h.state_mut().run_link_script(id, 0, 0);
        if !runs {
            assert_eq!(toast(h.state()), Some("Not allowed"), "said so, not the engine's Preferences hint");
        }
        settle(&mut h, |app| field(app, "log") == "clicked");
        assert_eq!(field(h.state(), "log") == "clicked", runs, "JavaScript allowed: {runs}");
    }
}

#[test]
fn ctrl_m_opens_the_comments_panel_only_with_comment_list() {
    let _gpu = gpu();
    for (profile, opens) in [(UiProfile::default(), true), (hiding(&["comment.list"]), false)] {
        let mut h = harness(profile, pdf("", &[]));
        // As Windows and Linux send it (upstream's own test): Ctrl sets `ctrl` and `command`.
        h.ctx.set_os(OperatingSystem::Windows);
        h.state_mut().right = None;
        h.run_steps(2);
        h.key_press_modifiers(Modifiers::CTRL | Modifiers::COMMAND, Key::M);
        h.run_steps(2);
        assert_eq!(h.state().right == Some(RightPanel::Comments), opens, "comment.list allowed: {opens}");
    }
}

/// A 1×1 transparent PNG.
const PNG: [u8; 67] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn a_picture_for_an_image_field_needs_form_fill() {
    let png = std::env::temp_dir().join(format!("fps-profile-photo-{}.png", std::process::id()));
    std::fs::write(&png, PNG).unwrap();
    let photo = "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (photo) /Rect [20 100 120 160] /P 3 0 R /F 4 >>";
    let every = UiProfile { features: Some(FeatureSet::all()), ..UiProfile::default() };
    for (profile, applies) in [(UiProfile::default(), true), (every, true), (without(&[Feature::FormFill]), false)] {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("doc.pdf", None, pdf("", &[photo])).unwrap();
        app.set_profile(profile);
        // The picker reads this file instead of asking (as other picker tests do).
        app.save_override = Some(png.display().to_string());
        let (_, id) = app.active_ids().unwrap();
        let before = app.session.get(id).unwrap().edit_generation();
        app.choose_field_image_target(id, "photo", None);
        let applied = app.session.get(id).unwrap().edit_generation() != before;
        assert_eq!(applied, applies, "form filling allowed: {applies}; {:?}", app.toast);
        if !applies {
            assert_eq!(toast(&app), Some("Not allowed"));
            continue;
        }
        // Applied to the document directly, it is still in the profile's undo log: Undo works
        // under a profile that limits features as under the default one.
        app.undo();
        assert!(app.session.get(id).unwrap().redo_label().is_some(), "undone; {:?}", app.toast);
    }
    let _ = std::fs::remove_file(&png);
}

/// A document with a button `go` whose action runs `script` (written for a PDF string).
fn scripted(script: &str) -> Vec<u8> {
    let button = format!(
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (go) /Rect [20 100 120 140] /P 3 0 R /F 4 /A << /S /JavaScript /JS ({script}) >> >>"
    );
    pdf("", &[&button])
}

#[test]
fn a_scripts_save_as_answers_to_file_save_as() {
    let _gpu = gpu();
    let cases = [
        (UiProfile::default(), true, "default"),
        (without(&[Feature::Download]), false, "no-download"),
        (hiding(&["file.save_as"]), false, "hidden"),
    ];
    for (profile, saves, name) in cases {
        let out = std::env::temp_dir().join(format!("fps-profile-saveas-{}-{name}.pdf", std::process::id()));
        let _ = std::fs::remove_file(&out);
        let mut h = harness(profile, scripted("app.execMenuItem\\('SaveAs'\\);"));
        h.state_mut().save_override = Some(out.display().to_string());
        let (_, id) = h.state().active_ids().unwrap();
        h.state_mut().run_button_script(id, "go", "app.execMenuItem('SaveAs');");
        settle(&mut h, |app| out.exists() || toast(app) == Some("Not allowed"));
        assert_eq!(out.exists(), saves, "{name}: {:?}", h.state().toast);
        if !saves {
            assert_eq!(toast(h.state()), Some("Not allowed"), "{name}");
        }
        let _ = std::fs::remove_file(&out);
    }
}

#[test]
fn a_scripts_open_doc_needs_tabs_and_file_open() {
    let _gpu = gpu();
    // An absolute path (the document has no folder); on Windows in Acrobat's `/C/...` form.
    let path = if cfg!(windows) { "/C/fps/other.pdf" } else { "/fps/other.pdf" };
    let script = format!("app.openDoc('{path}');");
    let cases = [(UiProfile::default(), true, "default"), (without(&[Feature::App]), false, "no-app"), (hiding(&["file.open"]), false, "hidden")];
    for (profile, asks, name) in cases {
        let mut h = harness(profile, pdf("", &[]));
        let (_, id) = h.state().active_ids().unwrap();
        h.state_mut().run_button_script(id, "go", &script);
        settle(&mut h, |app| app.pending_remote_link.is_some() || toast(app) == Some("Not allowed"));
        assert_eq!(h.state().pending_remote_link.is_some(), asks, "{name}: {:?}", h.state().toast);
        if !asks {
            assert_eq!(toast(h.state()), Some("Not allowed"), "{name}");
        }
    }
}

#[test]
fn the_arrow_keys_move_only_the_selected_comments_the_profile_allows() {
    use pdfcraft_engine::{Edit, NewAnnotation, Shape, Style};
    let _gpu = gpu();
    let add = |shape: Shape| {
        Edit::AddAnnotation(NewAnnotation { page: 0, style: Style::default_for(&shape), shape, contents: String::new(), author: "A".into() })
    };
    let mut no_ink = FeatureSet::all();
    no_ink.remove(Feature::Ink);
    for (features, ink_moves) in [(FeatureSet::all(), true), (no_ink, false)] {
        let mut h = harness(UiProfile::default(), pdf("", &[]));
        assert!(h.state_mut().apply_edit(add(Shape::Rectangle { rect: [30.0, 30.0, 60.0, 60.0] })));
        assert!(h.state_mut().apply_edit(add(Shape::Ink { strokes: vec![vec![[100.0, 100.0], [140.0, 140.0]]] })));
        h.state_mut().set_profile(UiProfile { features: Some(features), ..UiProfile::default() });
        let at = |h: &egui_kittest::Harness<'static, PdfCraftApp>, subtype: &str| {
            let (_, id) = h.state().active_ids().unwrap();
            let info = &h.state().session.get(id).unwrap().info;
            info.annotations.iter().find(|a| a.subtype == subtype).map(|a| (a.index, a.rect)).unwrap()
        };
        let ((square, square_at), (ink, ink_at)) = (at(&h, "Square"), at(&h, "Ink"));
        h.state_mut().views[0].comments.selected = Some((0, square));
        h.state_mut().views[0].comments.also = vec![(0, ink)];
        h.run_steps(2);
        h.key_press(Key::ArrowDown);
        h.run_steps(4);
        assert_ne!(at(&h, "Square").1, square_at, "the square moves (Ink allowed: {ink_moves})");
        assert_eq!(at(&h, "Ink").1 != ink_at, ink_moves, "the ink moves only with Ink allowed");
        assert!(h.query_by_label_contains("Not allowed").is_none(), "Ink allowed: {ink_moves}");
    }
}
