//! The UI profile on the canvas: form filling, form design, links, the page grid, the find bar
//! and the floating quick bar hide (and refuse) what the profile refuses; the default profile
//! changes nothing.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::features::Feature;
use pdfcraft_ui_egui::profile::{UiProfile, parts};
use pdfcraft_ui_egui::{LinkOrigin, PdfCraftApp, PendingLink};

/// See `tests/view.rs`: WARP's shader JIT crashes when two devices compile at once on ARM64.
static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gpu() -> std::sync::MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

const LINK_URL: &str = "https://example.org/from-a-link";
const BUTTON_URL: &str = "https://example.org/from-a-button";

/// Two 300×400 pages; the first has a web link (x 50..250, y 300..350), a push button with a URI
/// action ("web"), a push button with a script ("script") and a text field ("name").
fn fixture() -> Vec<u8> {
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R 6 0 R 7 0 R] /DA (/Helv 0 Tf 0 g) >> >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R 8 0 R] /Count 2 /MediaBox [0 0 300 400] >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>".to_string(),
        format!("<< /Type /Annot /Subtype /Link /Rect [50 300 250 350] /Border [0 0 0] /A << /S /URI /URI ({LINK_URL}) >> >>"),
        format!("<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (web) /Rect [20 200 100 220] /P 3 0 R /A << /S /URI /URI ({BUTTON_URL}) >> >>"),
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (script) /Rect [120 200 200 220] /P 3 0 R /AA << /U << /S /JavaScript /JS (var n = 6 * 7; app.alert\\(\"n\" + n\\);) >> >> >>".to_string(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /Rect [20 120 200 150] /P 3 0 R >>".to_string(),
        "<< /Type /Page /Parent 2 0 R >>".to_string(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offs = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offs.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let x = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offs {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}

fn harness(profile: UiProfile) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("gates.pdf", None, fixture()).unwrap();
        app.set_option("left", "closed").unwrap();
        app.set_option("panel", "none").unwrap();
        app.set_option("zoom", "150").unwrap();
        app.profile = profile;
        app
    });
    h.run_steps(8);
    h
}

/// Every feature but `off`.
fn without(off: &[Feature]) -> UiProfile {
    UiProfile { features: Some(Feature::ALL.into_iter().filter(|f| !off.contains(f)).collect()), ..UiProfile::default() }
}

fn hiding(ids: &[&str]) -> UiProfile {
    UiProfile { hidden: ids.iter().map(|s| s.to_string()).collect(), ..UiProfile::default() }
}

fn click_at(h: &mut Harness<'static, PdfCraftApp>, at: egui::Pos2) {
    h.hover_at(at);
    h.run_steps(2);
    h.drag_at(at);
    h.drop_at(at);
    h.run_steps(4);
}

fn click_link(h: &mut Harness<'static, PdfCraftApp>) {
    let r = h.state().views[0].page_screen_rect(0).expect("page 1 on screen");
    click_at(h, egui::pos2(r.min.x + r.width() * 0.5, r.min.y + r.height() * (1.0 - 325.0 / 400.0)));
}

fn click_field(h: &mut Harness<'static, PdfCraftApp>, name: &str) {
    let at = {
        let s = h.state();
        let doc = s.session.get(s.views[0].id).unwrap();
        let f = doc.form.iter().find(|f| f.name == name).unwrap();
        pdfcraft_ui_egui::forms_ui::field_screen_rect(&s.views[0], &doc.info, f, 0).expect("on screen").center()
    };
    click_at(h, at);
}

fn count(h: &Harness<'static, PdfCraftApp>, label: &str) -> usize {
    h.query_all_by_label(label).count()
}

#[test]
fn web_links_and_buttons_open_only_when_the_profile_allows_them() {
    let _gpu = gpu();
    let mut h = harness(UiProfile::default());
    click_link(&mut h);
    assert_eq!(h.state().pending_link, Some(PendingLink { url: LINK_URL.into(), origin: LinkOrigin::Link }));
    let mut h = harness(UiProfile::default());
    click_field(&mut h, "web");
    assert_eq!(h.state().pending_link, Some(PendingLink { url: BUTTON_URL.into(), origin: LinkOrigin::Button }));

    let mut h = harness(UiProfile { external_links: Some(false), ..UiProfile::default() });
    click_link(&mut h);
    assert_eq!(h.state().pending_link, None, "a link to a web page is refused");
    click_field(&mut h, "web");
    assert_eq!(h.state().pending_link, None, "and so is a button's");
    assert_eq!(h.state().last_opened_url, None);
}

#[test]
fn fields_are_inert_without_form_filling() {
    let _gpu = gpu();
    let mut h = harness(UiProfile::default());
    click_field(&mut h, "name");
    assert!(h.state().views[0].forms.focus.is_some(), "the default profile fills fields");

    let mut h = harness(without(&[Feature::FormFill]));
    click_field(&mut h, "name");
    assert!(h.state().views[0].forms.focus.is_none(), "no editor opens");
    assert!(h.state().toast.is_none(), "and nothing complains: the field is just not offered");
}

#[test]
fn a_button_script_does_not_run_without_javascript() {
    let _gpu = gpu();
    let mut h = harness(UiProfile::default());
    click_field(&mut h, "script");
    assert_eq!(h.state().toast.clone().map(|t| t.0).as_deref(), Some("n42"), "the default profile runs it");
    let mut h = harness(without(&[Feature::JavaScript]));
    click_field(&mut h, "script");
    let toast = h.state().toast.clone().expect("the user is told").0;
    assert_eq!(toast, "Not allowed");
}

#[test]
fn the_quick_bar_hides_as_a_part_and_keeps_fill_and_sign_while_any_fill_tool_is_allowed() {
    let _gpu = gpu();
    let shown = |profile: UiProfile| {
        let mut h = harness(profile);
        h.state_mut().set_option("mode", "edit").unwrap();
        h.run_steps(4);
        (count(&h, "Select (V)"), count(&h, "Fill & Sign"), count(&h, "Edit text & images"))
    };
    let default = shown(UiProfile::default());
    assert_eq!(default.0, 1, "the quick bar shows by default");
    let mut no_bar = UiProfile::default();
    no_bar.hidden.insert(parts::QUICK_BAR.to_string());
    let bare = shown(no_bar);
    assert_eq!(bare.0, 0, "hidden as a part");
    assert!(default.1 > bare.1 && default.2 > bare.2, "the bar has Fill & Sign and Edit text & images");
    // Hiding the first fill tool leaves the others.
    let no_text = shown(hiding(&["sign.fill.text"]));
    assert_eq!(no_text.1, default.1, "Fill & Sign stays while other fill tools are allowed");
    // Edit text & images goes with its command.
    let no_edit = shown(hiding(&["edit.edit_text"]));
    assert!(no_edit.2 < default.2, "Edit text & images is hidden with edit.edit_text");
}

#[test]
fn the_page_grid_hides_page_changes_without_the_pages_feature() {
    let _gpu = gpu();
    let grid = |profile: UiProfile| {
        let mut h = harness(profile);
        h.state_mut().views[0].organize = true;
        h.run_steps(4);
        (count(&h, "Move earlier"), count(&h, "Save pages"))
    };
    assert_eq!(grid(UiProfile::default()), (1, 1), "the default profile shows them");
    assert_eq!(grid(without(&[Feature::Pages])).0, 0, "no page moves without pages");
    assert_eq!(grid(UiProfile { read_only: true, ..UiProfile::default() }), (0, 0), "nor saving in view-only");
}

#[test]
fn the_find_bar_closes_when_find_is_refused() {
    let _gpu = gpu();
    for (profile, open) in [(UiProfile::default(), true), (hiding(&["edit.find"]), false)] {
        let mut h = harness(profile);
        h.state_mut().views[0].find = Some(Default::default());
        h.run_steps(3);
        assert_eq!(h.state().views[0].find.is_some(), open);
    }
}

/// One 300×200 page with a line of Helvetica text (as in `tests/comments.rs`).
const TEXT_FIXTURE: &[u8] = b"%PDF-1.7
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Contents 5 0 R /Resources << /Font << /F1 7 0 R >> >> >> endobj
5 0 obj << /Length 55 >> stream
BT /F1 14 Tf 20 150 Td (The quick brown fox) Tj ET
endstream endobj
7 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> endobj
trailer << /Root 1 0 R >>
%%EOF";

#[test]
fn the_selection_bar_shows_only_the_actions_the_profile_allows() {
    let _gpu = gpu();
    let bar = |profile: UiProfile| {
        let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
            let mut app = PdfCraftApp::new();
            app.set_option("language", "en").unwrap();
            app.open_bytes("words.pdf", None, TEXT_FIXTURE.to_vec()).unwrap();
            app.set_option("left", "closed").unwrap();
            app.set_option("panel", "none").unwrap();
            app.set_profile(profile);
            app
        });
        // Once the page is drawn and its text known.
        for _ in 0..200 {
            h.run_steps(2);
            if !h.state().render_pending() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut selected = false;
        for _ in 0..3000 {
            h.state_mut().views[0].select_text(0, 4, 8);
            h.run_steps(2);
            if h.state().views[0].selected_text().is_some() {
                selected = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(selected, "the page has text to select");
        h.run_steps(3);
        (count(&h, "Highlight"), count(&h, "Copy"), count(&h, "Add comment"))
    };
    // Highlight is on the quick bar too.
    assert_eq!(bar(UiProfile::default()), (2, 1, 1));
    assert_eq!(bar(hiding(&["comment.highlight", "edit.copy"])), (0, 0, 1));
}
