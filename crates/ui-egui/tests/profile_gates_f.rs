//! The UI profile on what pdfcraft added after 0.18.1: split view, and copying and pasting
//! comments. What the profile refuses is refused however it is asked for; the default profile
//! changes nothing.

use pdfcraft_engine::{Edit, NewAnnotation, Shape, Style};
use pdfcraft_ui_egui::PdfCraftApp;
use pdfcraft_ui_egui::features::Feature;
use pdfcraft_ui_egui::profile::UiProfile;

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

/// A two-page document under `profile`.
fn app(profile: UiProfile) -> PdfCraftApp {
    let mut app = PdfCraftApp::new();
    app.set_option("language", "en").unwrap();
    app.open_bytes("doc.pdf", None, fixture(2)).unwrap();
    app.set_profile(profile);
    app
}

/// [`app`] with one rectangle comment, selected (added before the profile is set).
fn with_comment(profile: UiProfile) -> PdfCraftApp {
    let mut app = app(UiProfile::default());
    assert!(app.apply_edit(rectangle()));
    let (_, id) = app.active_ids().unwrap();
    let index = app.session.get(id).unwrap().info.annotations[0].index;
    app.views[0].comments.selected = Some((0, index));
    app.set_profile(profile);
    app
}

fn comments(app: &PdfCraftApp) -> usize {
    let (_, id) = app.active_ids().unwrap();
    app.session.get(id).unwrap().info.annotations.len()
}

fn hiding(ids: &[&str]) -> UiProfile {
    UiProfile { hidden: ids.iter().map(|s| s.to_string()).collect(), ..UiProfile::default() }
}

/// Every feature but `off`.
fn without(off: &[Feature]) -> UiProfile {
    UiProfile { features: Some(Feature::ALL.into_iter().filter(|f| !off.contains(f)).collect()), ..UiProfile::default() }
}

/// The copied comments of a document with one comment, as XFDF.
fn copied() -> String {
    let mut source = with_comment(UiProfile::default());
    assert!(source.copy_selected_comments(0));
    source.comment_clipboard.as_ref().unwrap().xfdf.clone()
}

#[test]
fn copying_comments_needs_comment_copy() {
    let mut app = with_comment(UiProfile::default());
    assert!(app.copy_selected_comments(0), "the default profile copies the selected comment");
    assert!(app.comment_clipboard.is_some());

    // ⌘C goes through copy_selected_comments too: refused, it leaves the key to the page's text.
    let mut app = with_comment(hiding(&["comment.copy"]));
    assert!(!app.copy_selected_comments(0));
    app.execute("comment.copy");
    assert!(app.comment_clipboard.is_none(), "nothing reaches the clipboard");
}

#[test]
fn pasting_comments_needs_comment_paste_and_comments() {
    let xfdf = copied();
    let at = Some((0, [100.0, 150.0]));

    let mut app = with_comment(UiProfile::default());
    assert!(app.paste_comments(0, Some(xfdf.clone()), at));
    assert_eq!(comments(&app), 2, "the default profile pastes the copy");

    // A pasted XFDF text and the page's menu both come through paste_comments.
    let mut app = with_comment(hiding(&["comment.paste"]));
    assert!(!app.paste_comments(0, Some(xfdf.clone()), at));
    assert_eq!(comments(&app), 1, "hidden by id");

    let mut app = with_comment(without(&[Feature::Comments]));
    assert!(!app.paste_comments(0, Some(xfdf), at));
    assert_eq!(comments(&app), 1, "refused with the comments feature");
}

#[test]
fn split_view_is_the_apps() {
    // The SDK never turns the app feature on: split view is one of its tab layouts.
    let mut refused = app(without(&[Feature::App]));
    refused.execute("view.split");
    assert!(refused.panes.split.is_none(), "no split without the app feature");

    let mut hidden = app(hiding(&["view.split"]));
    hidden.execute("view.split");
    assert!(hidden.panes.split.is_none(), "no split when the command is hidden");

    let mut default = app(UiProfile::default());
    default.execute("view.split");
    assert!(default.panes.split.is_some(), "the default profile splits");
}

#[test]
fn pasted_comments_need_each_kind_they_hold() {
    // The copied comment is a rectangle: a shape.
    let xfdf = copied();
    let at = Some((0, [100.0, 150.0]));
    let mut app = with_comment(without(&[Feature::Shapes]));
    assert!(!app.paste_comments(0, Some(xfdf.clone()), at), "comments on, shapes off");
    assert_eq!(comments(&app), 1);
    let mut app = with_comment(UiProfile { features: Some([Feature::Comments, Feature::Shapes].into_iter().collect()), ..UiProfile::default() });
    assert!(app.paste_comments(0, Some(xfdf), at));
    assert_eq!(comments(&app), 2, "comments and shapes on");
}

#[test]
fn comments_only_takes_the_new_comment_edits() {
    let xfdf = copied();
    let mut app = with_comment(UiProfile { comments_only: true, ..UiProfile::default() });
    assert!(app.paste_comments(0, Some(xfdf), Some((0, [100.0, 150.0]))), "a paste is a comment edit");
    let (_, id) = app.active_ids().unwrap();
    let index = app.session.get(id).unwrap().info.annotations[0].index;
    assert!(app.apply_edit(Edit::SetAnnotationPopup { page: 0, index, rect: None, open: Some(true) }), "so is opening a pop-up");
    assert!(app.apply_edit(Edit::PartOpacity { page: 0, index, fill: Some(0.5), text: None }), "and a part's opacity");
}

/// See `tests/view.rs`: WARP's shader JIT crashes when two devices compile at once on ARM64.
static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gpu() -> std::sync::MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The app with `pdf` opened under `profile`, after a few frames.
fn harness(profile: UiProfile, pdf: Vec<u8>) -> egui_kittest::Harness<'static, PdfCraftApp> {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.set_profile(profile);
        app.open_bytes("portfolio.pdf", None, pdf).unwrap();
        app
    });
    h.run_steps(6);
    h
}

/// A one-page portfolio presenting `form.pdf` (`/Collection /D`), a one-page PDF.
fn portfolio() -> Vec<u8> {
    let page = fixture(1);
    let objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R /Collection << /D (form.pdf) /View /T >> /Names << /EmbeddedFiles << /Names [(form.pdf) 5 0 R] >> >> >>"
            .to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> >>".to_vec(),
        [format!("<< /Type /EmbeddedFile /Length {} >>\nstream\n", page.len()).as_bytes(), &page, b"\nendstream"].concat(),
        b"<< /Type /Filespec /F (form.pdf) /UF (form.pdf) /EF << /F 4 0 R >> >>".to_vec(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}

/// The names of the open documents, and the active one's.
fn tabs(h: &egui_kittest::Harness<'static, PdfCraftApp>) -> (Vec<String>, Option<String>) {
    let s = h.state();
    let name = |i: usize| s.views.get(i).and_then(|v| s.session.get(v.id)).map(|d| d.name.clone());
    ((0..s.views.len()).filter_map(name).collect(), s.active.and_then(name))
}

#[test]
fn a_portfolio_opens_its_initial_document_only_where_documents_open_in_tabs() {
    let _gpu = gpu();
    let h = harness(UiProfile::default(), portfolio());
    assert_eq!(tabs(&h), (vec!["portfolio.pdf".into(), "form.pdf".into()], Some("form.pdf".into())), "the default profile presents it");
    // The SDK's profiles leave the app off: the portfolio stays the document shown (and the one
    // the host works on).
    let h = harness(without(&[Feature::App]), portfolio());
    assert_eq!(tabs(&h), (vec!["portfolio.pdf".into()], Some("portfolio.pdf".into())));
}

#[test]
fn the_quick_bar_crop_button_needs_page_organization() {
    let _gpu = gpu();
    let crop = |h: &egui_kittest::Harness<'static, PdfCraftApp>| {
        use egui_kittest::kittest::Queryable;
        h.query_all_by_label("Crop pages").count()
    };
    assert!(crop(&harness(UiProfile::default(), fixture(2))) > 0, "the default profile offers it");
    assert_eq!(crop(&harness(without(&[Feature::Pages]), fixture(2))), 0, "hidden, not greyed");
}

#[test]
fn a_pasted_reply_of_another_kind_needs_its_kind_too() {
    // A "reply" whose comment isn't pasted with it stands alone, as the square it is.
    let xfdf = r#"<?xml version="1.0"?><xfdf xmlns="http://ns.adobe.com/xfdf/"><annots><text page="0" rect="10,10,30,30" name="t"/><square page="0" rect="50,50,90,90" name="s" inreplyto="gone"/></annots></xfdf>"#;
    let review = UiProfile { features: Some([Feature::Comments, Feature::Markup].into_iter().collect()), ..UiProfile::default() };
    let mut app = with_comment(review);
    assert!(!app.paste_comments(0, Some(xfdf.to_owned()), Some((0, [100.0, 150.0]))), "comments and markup only");
    // Nor when the edit reaches apply_edit another way.
    assert!(!app.apply_edit(Edit::PasteComments { xfdf: xfdf.to_owned(), page: 0, offset: Some([0.0, 0.0]) }));
    assert_eq!(comments(&app), 1);
}
