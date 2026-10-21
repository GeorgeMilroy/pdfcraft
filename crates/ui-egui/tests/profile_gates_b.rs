//! The UI profile in dialogs, Preferences, printing, signing and accessibility: what the profile
//! refuses is left out (not greyed) and refused if reached anyway; the default profile shows
//! everything as before.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_engine::{Edit, NewAnnotation, Shape, Style};
use pdfcraft_ui_egui::features::Feature;
use pdfcraft_ui_egui::profile::UiProfile;
use pdfcraft_ui_egui::{CloseRequest, Dialog, PdfCraftApp, PropsTab, RightPanel};

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

fn rectangle() -> Edit {
    let shape = Shape::Rectangle { rect: [10.0, 10.0, 50.0, 50.0] };
    Edit::AddAnnotation(NewAnnotation { page: 0, style: Style::default_for(&shape), shape, contents: "note".into(), author: "A".into() })
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
    h
}

/// A profile offering only `features`.
fn only(features: &[Feature]) -> UiProfile {
    UiProfile { features: Some(features.iter().copied().collect()), ..UiProfile::default() }
}

fn shows(h: &Harness<'_, PdfCraftApp>, label: &str) -> bool {
    h.query_all_by_label(label).next().is_some()
}

fn open(h: &mut Harness<'_, PdfCraftApp>, dialog: Dialog) {
    h.state_mut().dialog = Some(dialog);
    h.run_steps(3);
}

#[test]
fn preferences_show_the_sections_of_the_features_kept() {
    let mut h = harness(UiProfile::default());
    open(&mut h, Dialog::Preferences);
    for label in ["Enable Acrobat JavaScript", "Flatten Fill & Sign when saving", "Name on new comments"] {
        assert!(shows(&h, label), "{label} shows by default");
    }
    assert!(h.query_by_role_and_label(Role::RadioButton, "E-Sign").is_some(), "every default mode by default");

    let mut h = harness(only(&[Feature::Preferences, Feature::Comments]));
    open(&mut h, Dialog::Preferences);
    for label in ["Enable Acrobat JavaScript", "Flatten Fill & Sign when saving", "Hide hover previews for highlights without comment text"] {
        assert!(!shows(&h, label), "{label} shows without its feature");
    }
    assert!(shows(&h, "Name on new comments"), "comments keep their author");
    for mode in ["Edit", "Convert", "E-Sign"] {
        assert!(h.query_by_role_and_label(Role::RadioButton, mode).is_none(), "{mode} offered as the default mode");
    }
    assert!(h.query_by_role_and_label(Role::RadioButton, "Read").is_some());
}

#[test]
fn scripts_do_not_run_without_javascript() {
    let run = |profile: UiProfile| {
        let mut h = harness(profile);
        h.state_mut().js_console.input = "console.println('ran')".into();
        h.state_mut().run_console();
        h.state().js_console.log.clone()
    };
    assert!(!run(UiProfile::default()).is_empty(), "the console runs by default");
    assert!(run(only(&[Feature::Comments])).is_empty(), "nothing runs without javascript");
}

#[test]
fn printing_and_save_as_pdf_follow_their_features() {
    let mut h = harness(only(&[Feature::Comments]));
    h.state_mut().open_print();
    assert_eq!(h.state().dialog, None, "no Print dialog without print");

    // Save as PDF writes a copy: offered only with download.
    let label = |profile: UiProfile| {
        let mut h = harness(profile);
        h.state_mut().open_print();
        assert_eq!(h.state().dialog, Some(Dialog::Print));
        h.state_mut().print_draft.printer = None;
        h.state_mut().print_draft.printers_loading = false;
        h.run_steps(3);
        shows(&h, "Save as PDF")
    };
    assert!(label(UiProfile::default()));
    assert!(!label(only(&[Feature::Print])));
}

#[test]
fn signing_needs_digital_signatures() {
    let mut h = harness(UiProfile::default());
    h.state_mut().start_signing(0, Some([10.0, 10.0, 100.0, 50.0]), None, None);
    assert_eq!(h.state().dialog, Some(Dialog::Sign));
    for profile in [only(&[Feature::SignaturePanel, Feature::FillSign]), UiProfile { read_only: true, ..UiProfile::default() }] {
        let mut h = harness(profile);
        h.state_mut().start_signing(0, Some([10.0, 10.0, 100.0, 50.0]), None, None);
        assert_eq!(h.state().dialog, None, "signing started without digital signatures");
        assert!(h.state().sign_draft.is_none());
    }
}

#[test]
fn properties_only_show_without_content_editing() {
    let changed = |profile: UiProfile| {
        let mut h = harness(profile);
        open(&mut h, Dialog::Properties(PropsTab::Description));
        h.state_mut().props_draft.as_mut().unwrap().1[0] = "A new title".into();
        h.run_steps(3);
        (shows(&h, "OK"), shows(&h, "Close"), h.query_by_role_and_label(Role::TextInput, "Title").is_some())
    };
    assert_eq!(changed(UiProfile::default()), (true, false, true), "editable by default");
    assert_eq!(changed(only(&[Feature::DocumentInfo])), (false, true, false), "only shown without editContent");
    assert_eq!(changed(UiProfile { read_only: true, ..UiProfile::default() }), (false, true, false));

    // Security: Protect needs the security feature.
    let protect = |profile: UiProfile| {
        let mut h = harness(profile);
        open(&mut h, Dialog::Properties(PropsTab::Security));
        shows(&h, "Protect using password…")
    };
    assert!(protect(UiProfile::default()));
    assert!(!protect(only(&[Feature::DocumentInfo])));
}

#[test]
fn shortcuts_list_only_what_the_profile_runs() {
    let list = |profile: UiProfile| {
        let mut h = harness(profile);
        open(&mut h, Dialog::Shortcuts);
        (shows(&h, "Delete selected pages (Organize)"), h.query_all_by_label_contains("Undo").next().is_some())
    };
    assert_eq!(list(UiProfile::default()), (true, true));
    assert_eq!(list(UiProfile { read_only: true, ..UiProfile::default() }), (false, false));
}

#[test]
fn the_save_prompt_offers_save_only_when_saving_is_allowed() {
    let prompt = |read_only: bool| {
        let mut h = harness(UiProfile::default());
        assert!(h.state_mut().apply_edit(rectangle()));
        let id = h.state().views[0].id;
        h.state_mut().profile.read_only = read_only;
        h.state_mut().close_request = Some(CloseRequest::Tab(id));
        h.run_steps(3);
        assert!(shows(&h, "Don't save") && shows(&h, "Cancel"));
        shows(&h, "Save")
    };
    assert!(prompt(false));
    assert!(!prompt(true));
}

#[test]
fn without_save_the_prompt_keys_choose_only_what_is_offered() {
    let prompt = || {
        let mut h = harness(UiProfile::default());
        assert!(h.state_mut().apply_edit(rectangle()));
        let id = h.state().views[0].id;
        h.state_mut().profile.read_only = true;
        h.state_mut().close_request = Some(CloseRequest::Tab(id));
        h.run_steps(3);
        assert!(!shows(&h, "Save"));
        h
    };
    let chosen = |h: &Harness<'_, PdfCraftApp>, label: &str| h.get_by_label(label).is_focused();
    // The choice starts on Cancel, and moving toward where Save would be keeps it there: Enter
    // cancels, and nothing is saved.
    for keys in [vec![], vec![egui::Key::ArrowRight], vec![egui::Key::ArrowDown, egui::Key::ArrowRight]] {
        let mut h = prompt();
        for key in &keys {
            h.key_press(*key);
            h.run_steps(1);
        }
        assert!(chosen(&h, "Cancel"), "{keys:?}: Cancel is the choice");
        h.key_press(egui::Key::Enter);
        h.run_steps(3);
        assert_eq!(h.state().views.len(), 1, "{keys:?} then Enter keeps the document");
        assert!(h.state().close_request.is_none(), "{keys:?} then Enter cancels");
        assert!(h.state().first_dirty().is_some(), "{keys:?} then Enter saves nothing");
    }
    // Tab skips the missing Save: from Cancel it reaches Don't save, which closes without saving.
    let mut h = prompt();
    h.key_press(egui::Key::Tab);
    h.run_steps(1);
    assert!(chosen(&h, "Don't save"));
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert!(h.state().views.is_empty(), "Tab then Enter is Don't save");
}

#[test]
fn the_accessibility_report_is_a_copy() {
    let report = |profile: UiProfile| {
        let mut h = harness(profile);
        h.state_mut().run_accessibility_check();
        h.run_steps(3);
        assert_eq!(h.state().right, Some(RightPanel::Accessibility));
        shows(&h, "Report")
    };
    assert!(report(UiProfile::default()));
    assert!(!report(only(&[Feature::Accessibility])), "the report writes a file: download");
}
