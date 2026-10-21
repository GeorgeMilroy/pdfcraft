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

#[test]
fn signature_apply_arms_the_tool_as_before_unless_the_profile_refuses_it() {
    let apply = |profile: UiProfile| {
        // No document: the default profile armed the tool all the same.
        let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
            let mut app = PdfCraftApp::new();
            app.set_option("language", "en").unwrap();
            app.set_profile(profile);
            app.set_option("dialog", "signature").unwrap();
            app.signature_draft.text = "Ada Lovelace".into();
            app
        });
        h.run_steps(3);
        h.get_by_label("Apply").click();
        h.run_steps(3);
        assert!(h.state().signature.is_some(), "the signature is saved either way");
        h.state().quick_tool
    };
    let signature = pdfcraft_ui_egui::QuickTool::Fill(pdfcraft_ui_egui::fill_sign::FillTool::Signature);
    assert_eq!(apply(UiProfile::default()), signature);
    // The dialog stays open for the change command, but placing the signature is refused.
    let refused = UiProfile { hidden: ["sign.fill.signature".to_string()].into(), ..UiProfile::default() };
    assert_ne!(apply(refused), signature);
}

#[test]
fn scripts_ask_for_nothing_without_javascript() {
    use pdfcraft_engine::js::{JsOutput, Request};
    let output = || JsOutput { alerts: vec!["from a script".into()], requests: vec![Request::GoToPage(1)], ..JsOutput::default() };
    let mut h = harness(UiProfile::default());
    let id = h.state().views[0].id;
    h.state_mut().handle_js(id, output());
    assert_eq!(h.state().toast.clone().map(|t| t.0).as_deref(), Some("from a script"), "the default profile shows the alert");

    let mut h = harness(UiProfile::default());
    let id = h.state().views[0].id;
    h.state_mut().set_profile(only(&[Feature::Viewer]));
    assert!(!h.state().session.javascript(), "documents' scripts are held off");
    h.state_mut().handle_js(id, output());
    h.run_steps(2);
    assert!(h.state().toast.is_none(), "no alert");
    assert_eq!(h.state().views[0].current, 0, "no navigation");
    h.state_mut().set_profile(UiProfile::default());
    assert!(h.state().session.javascript(), "the user's preference comes back with the feature");
}

#[test]
fn the_accessibility_panel_follows_a_profile_change_on_the_same_frame() {
    let mut h = harness(UiProfile::default());
    h.state_mut().set_option("panel", "accessibility").unwrap();
    h.run_steps(3);
    assert!(shows(&h, "Check for accessibility"));
    h.state_mut().set_profile(UiProfile { hidden: ["a11y.check".to_string()].into(), ..UiProfile::default() });
    h.run_steps(1);
    assert!(!shows(&h, "Check for accessibility"), "not drawn with last frame's profile");
    // After a check, Check again needs a11y.check too.
    h.state_mut().set_profile(UiProfile::default());
    h.state_mut().run_accessibility_check();
    h.run_steps(3);
    assert!(shows(&h, "Check again"));
    h.state_mut().set_profile(UiProfile { hidden: ["a11y.check".to_string()].into(), ..UiProfile::default() });
    h.run_steps(1);
    assert!(!shows(&h, "Check again"));
}

#[test]
fn a_script_opens_no_web_page_when_the_profile_turns_links_off() {
    use pdfcraft_engine::js::{JsOutput, Request};
    let launch = || JsOutput { requests: vec![Request::LaunchUrl("https://example.org/".into())], ..JsOutput::default() };
    let mut h = harness(UiProfile::default());
    let id = h.state().views[0].id;
    h.state_mut().handle_js(id, launch());
    assert!(h.state().pending_link.is_some(), "the default profile asks the user");
    let mut h = harness(UiProfile { external_links: Some(false), ..UiProfile::default() });
    let id = h.state().views[0].id;
    h.state_mut().handle_js(id, launch());
    assert!(h.state().pending_link.is_none());
    assert_eq!(h.state().toast.clone().map(|t| t.0).as_deref(), Some("Not allowed"));
}

fn hiding(ids: &[&str]) -> UiProfile {
    UiProfile { hidden: ids.iter().map(|s| s.to_string()).collect(), ..UiProfile::default() }
}

fn toast(h: &Harness<'_, PdfCraftApp>) -> Option<String> {
    h.state().toast.clone().map(|t| t.0)
}

#[test]
fn approval_signing_and_certifying_are_each_their_own_command() {
    let rect = Some([10.0, 10.0, 100.0, 50.0]);
    // (hidden, certify refused, approval refused)
    for (hidden, certify_refused, sign_refused) in [("sign.digital", false, true), ("sign.certify", true, false)] {
        for (certify, refused) in [(Some(2), certify_refused), (None, sign_refused)] {
            let mut h = harness(hiding(&[hidden]));
            h.state_mut().start_signing(0, rect, None, certify);
            assert_eq!(h.state().dialog.is_none(), refused, "{hidden} hidden, certify {certify:?}");
            assert_eq!(h.state().sign_draft.is_none(), refused);
            assert_eq!(toast(&h).as_deref() == Some("Not allowed"), refused, "a refusal says so");
        }
    }
    // An open Sign dialog closes when the command its draft is for goes, and only then.
    for (certify, hidden, open) in
        [(None, "sign.digital", false), (Some(2), "sign.digital", true), (Some(2), "sign.certify", false), (None, "sign.certify", true)]
    {
        let mut h = harness(UiProfile::default());
        h.state_mut().start_signing(0, rect, None, certify);
        h.state_mut().set_profile(hiding(&[hidden]));
        assert_eq!(h.state().dialog == Some(Dialog::Sign), open, "certify {certify:?} with {hidden} hidden");
    }
}

/// One page with an empty signature field.
const WITH_FIELD: &[u8] = b"%PDF-1.7
1 0 obj << /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [6 0 R] >> >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Annots [6 0 R] >> endobj
6 0 obj << /Type /Annot /Subtype /Widget /FT /Sig /T (Approver) /Rect [20 20 280 80] /P 3 0 R /F 4 >> endobj
trailer << /Root 1 0 R >>
%%EOF";

#[test]
fn sign_this_field_needs_approval_signing() {
    let offered = |profile: UiProfile| {
        let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
            let mut app = PdfCraftApp::new();
            app.set_option("language", "en").unwrap();
            app.open_bytes("form.pdf", None, WITH_FIELD.to_vec()).unwrap();
            app.set_profile(profile);
            app.set_option("panel", "signatures").unwrap();
            app.sig_expanded = vec!["Approver".into()];
            app
        });
        h.run_steps(6);
        assert!(shows(&h, "Unsigned signature field: Approver"), "the panel lists the field");
        shows(&h, "Sign this field…")
    };
    assert!(offered(UiProfile::default()));
    assert!(offered(hiding(&["sign.certify"])), "certifying is another command");
    assert!(!offered(hiding(&["sign.digital"])));
}

#[test]
fn properties_tabs_follow_their_own_commands() {
    let tabs = |profile: UiProfile, tab: PropsTab| {
        let mut h = harness(profile);
        open(&mut h, Dialog::Properties(tab));
        ["Description", "Initial View", "Security", "Fonts", "Advanced"].map(|l| h.query_all_by_label(l).count())
    };
    let all = tabs(UiProfile::default(), PropsTab::Description);
    assert!(all.iter().all(|n| *n > 0), "every tab by default: {all:?}");
    let no_security = tabs(hiding(&["protect.properties"]), PropsTab::Description);
    assert_eq!(no_security[2], all[2] - 1, "no Security tab");
    assert_eq!([no_security[0], no_security[1], no_security[3], no_security[4]], [all[0], all[1], all[3], all[4]]);
    let all = tabs(UiProfile::default(), PropsTab::Security);
    let only_security = tabs(hiding(&["file.properties"]), PropsTab::Security);
    assert_eq!(only_security, [all[0] - 1, all[1] - 1, all[2], all[3] - 1, all[4] - 1], "only the Security tab");

    // A request for a refused tab is refused, and changes nothing.
    let mut h = harness(hiding(&["protect.properties"]));
    assert!(h.state_mut().set_option("dialog", "security").is_err());
    assert_eq!(h.state().dialog, None);
    h.state_mut().set_option("dialog", "fonts").unwrap();
    assert_eq!(h.state().dialog, Some(Dialog::Properties(PropsTab::Fonts)));
    let mut h = harness(hiding(&["file.properties"]));
    for refused in ["properties", "fonts", "advanced"] {
        assert!(h.state_mut().set_option("dialog", refused).is_err(), "{refused}");
    }
    h.state_mut().set_option("dialog", "security").unwrap();
    assert_eq!(h.state().dialog, Some(Dialog::Properties(PropsTab::Security)));

    // A tab the profile comes to refuse gives way to one it allows.
    let mut h = harness(UiProfile::default());
    open(&mut h, Dialog::Properties(PropsTab::Security));
    h.state_mut().set_profile(hiding(&["protect.properties"]));
    assert_eq!(h.state().dialog, Some(Dialog::Properties(PropsTab::Description)));
    h.state_mut().dialog = Some(Dialog::Properties(PropsTab::Fonts));
    h.state_mut().set_profile(hiding(&["file.properties"]));
    assert_eq!(h.state().dialog, Some(Dialog::Properties(PropsTab::Security)));
    h.state_mut().set_profile(hiding(&["file.properties", "protect.properties"]));
    assert_eq!(h.state().dialog, None);
}

/// A host like free-pdf-sdk's: it runs the app's `logic`, then sets the profile, then draws.
struct Host {
    app: PdfCraftApp,
    next: Option<UiProfile>,
}

impl eframe::App for Host {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.logic(ctx, frame);
        if let Some(profile) = self.next.take() {
            self.app.set_profile(profile);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
    }
}

#[test]
fn a_profile_set_after_logic_is_drawn_in_the_same_frame() {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("doc.pdf", None, fixture(2)).unwrap();
        app.set_option("panel", "accessibility").unwrap();
        Host { app, next: None }
    });
    h.run_steps(4);
    assert!(h.query_all_by_label("Check for accessibility").next().is_some());
    h.state_mut().next = Some(hiding(&["a11y.check"]));
    h.run_steps(1);
    assert!(h.query_all_by_label("Check for accessibility").next().is_none(), "not drawn with last frame's profile");
    h.state_mut().next = Some(UiProfile::default());
    h.run_steps(1);
    assert!(h.query_all_by_label("Check for accessibility").next().is_some(), "and back");
}
