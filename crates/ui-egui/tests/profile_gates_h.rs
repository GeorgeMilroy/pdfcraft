//! Review findings from the update to pdfcraft 0.21: the default profile changes nothing (Home's
//! Recommended tools heading, an unreadable paste, cutting pages), and a Hide action fills the
//! form rather than running JavaScript.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;
use pdfcraft_ui_egui::features::Feature;
use pdfcraft_ui_egui::profile::UiProfile;

/// A PDF of `objs` (object 1 the catalog).
fn pdf(objs: &[String]) -> Vec<u8> {
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

/// `n` blank pages.
fn pages(n: usize) -> Vec<u8> {
    let mut objs: Vec<String> = vec!["<< /Type /Catalog /Pages 2 0 R >>".into()];
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 3 + i)).collect();
    objs.push(format!("<< /Type /Pages /Kids [{}] /Count {n} /MediaBox [0 0 200 300] >>", kids.join(" ")));
    for _ in 0..n {
        objs.push("<< /Type /Page /Parent 2 0 R >>".into());
    }
    pdf(&objs)
}

/// A text field `name` and two buttons whose PDF Hide actions hide it ("hide") and show it again
/// ("show"); no script anywhere.
fn show_hide() -> Vec<u8> {
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R 6 0 R] /DA (/Helv 0 Tf 0 g) >> >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 300 400] >>".into(),
        "<< /Type /Page /Parent 2 0 R /Annots [4 0 R 5 0 R 6 0 R] >>".into(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /V (Ada) /F 4 /Rect [20 340 280 360] /P 3 0 R >>".into(),
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (hide) /Rect [20 300 100 320] /P 3 0 R /A << /S /Hide /T (name) >> >>".into(),
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (show) /Rect [120 300 200 320] /P 3 0 R /A << /S /Hide /T 4 0 R /H false >> >>"
            .into(),
    ])
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

#[test]
fn home_shows_recommended_tools_however_many_are_pinned() {
    let _gpu = gpu();
    let recommended = ["organize", "comment", "form", "edit", "protect"];
    let groups: Vec<String> = recommended.iter().map(|g| format!("group:{g}")).collect();
    let groups: Vec<&str> = groups.iter().map(String::as_str).collect();
    for (profile, heading) in [(UiProfile::default(), true), (hiding(&groups), false)] {
        let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
            let mut app = PdfCraftApp::new();
            app.set_option("language", "en").unwrap();
            app.set_profile(profile);
            // Every recommended tool pinned: their cards move to the pinned row.
            app.pinned_tools = recommended.iter().map(|g| (*g).to_string()).collect();
            app
        });
        h.run_steps(4);
        assert_eq!(h.query_by_label("Recommended tools").is_some(), heading, "a recommended group offered: {heading}");
    }
}

#[test]
fn an_unreadable_paste_gets_upstreams_answer() {
    // Looks like copied comments (an `<xfdf` head and `<annots`) but isn't XML that reads.
    let broken = r#"<xfdf xmlns="http://ns.adobe.com/xfdf/"><annots><square page="0" rect="0,0,10,10"></xfdf>"#;
    let mut limited = Feature::ALL.into_iter().collect::<pdfcraft_ui_egui::features::FeatureSet>();
    limited.remove(Feature::Ink);
    for profile in [UiProfile::default(), UiProfile { features: Some(limited), ..UiProfile::default() }] {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("doc.pdf", None, pages(1)).unwrap();
        app.set_profile(profile);
        assert!(!app.paste_comments(0, Some(broken.into()), None));
        assert_eq!(toast(&app), Some("The clipboard holds no comments"));
    }
}

#[test]
fn cutting_pages_keeps_upstreams_order_unless_the_profile_refuses() {
    // A document whose security allows no changes.
    let mut source = PdfCraftApp::new();
    source.open_bytes("doc.pdf", None, pages(3)).unwrap();
    assert!(source.apply_edit(pdfcraft_engine::Edit::Protect(pdfcraft_engine::Protection {
        permissions_password: Some("owner".into()),
        changes: pdfcraft_engine::Changes::None,
        ..Default::default()
    })));
    let restricted = source.session.save_bytes(source.views[0].id).unwrap().as_ref().clone();
    for (profile, copied) in [(UiProfile::default(), true), (without(&[Feature::Pages]), false), (hiding(&["page.delete"]), true)] {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("restricted.pdf", None, restricted.clone()).unwrap();
        app.set_profile(profile);
        app.copy_pages(true);
        // pdfcraft as it ships keeps the pages on the clipboard though the document refuses the
        // deletion; a profile that refuses the cut leaves the clipboard as it was.
        assert_eq!(app.page_clipboard.is_some(), copied, "{:?}", app.toast);
        let (_, id) = app.active_ids().unwrap();
        assert_eq!(app.session.get(id).unwrap().info.pages.len(), 3, "no page is deleted");
        if !copied {
            assert_eq!(toast(&app), Some("Not allowed"));
        }
    }
}

fn click_field(h: &mut Harness<'static, PdfCraftApp>, name: &str) {
    let p = {
        let s = h.state();
        let doc = s.session.get(s.views[0].id).unwrap();
        let f = doc.form.iter().find(|f| f.name == name).unwrap();
        pdfcraft_ui_egui::forms_ui::field_screen_rect(&s.views[0], &doc.info, f, 0).expect("on screen").center()
    };
    h.hover_at(p);
    h.run_steps(1);
    h.drag_at(p);
    h.run_steps(1);
    h.drop_at(p);
    h.run_steps(3);
}

/// Whether field `name` (object 4) is hidden in the saved bytes.
fn name_hidden(h: &Harness<'static, PdfCraftApp>) -> bool {
    let s = h.state();
    let bytes = s.session.save_bytes(s.views[0].id).unwrap();
    let doc = pdfcraft_cos::Document::open(bytes).unwrap();
    doc.get(pdfcraft_cos::ObjRef::new(4, 0)).as_dict().and_then(|d| d.int(b"F")).unwrap_or(0) & 2 != 0
}

#[test]
fn a_hide_action_needs_form_filling_not_javascript() {
    let _gpu = gpu();
    // The SDK's Fill mode: form filling without JavaScript.
    for (profile, hides) in [(without(&[Feature::JavaScript]), true), (without(&[Feature::FormFill]), false)] {
        let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
            let mut app = PdfCraftApp::new();
            app.set_option("language", "en").unwrap();
            app.open_bytes("show-hide.pdf", None, show_hide()).unwrap();
            app.set_option("zoom", "150").unwrap();
            app.set_profile(profile);
            app
        });
        h.run_steps(6);
        click_field(&mut h, "hide");
        assert_eq!(name_hidden(&h), hides, "form filling allowed: {hides}; {:?}", h.state().toast);
        if hides {
            click_field(&mut h, "show");
            assert!(!name_hidden(&h), "and shown again");
        } else {
            assert_eq!(toast(h.state()), Some("Not allowed"));
        }
    }
}

#[test]
fn home_draws_no_empty_card_when_every_offered_tool_is_pinned_and_open_is_refused() {
    let _gpu = gpu();
    let recommended = ["organize", "comment", "form", "edit", "protect"];
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.set_profile(hiding(&["file.open"]));
        app.pinned_tools = recommended.iter().map(|g| (*g).to_string()).collect();
        app
    });
    h.run_steps(4);
    // Nothing left for the card to hold: no card, so no heading over an empty one.
    assert!(h.query_by_label("Recommended tools").is_none());
    assert!(h.query_by_label("Open file").is_none());
}

#[test]
fn detecting_form_fields_can_be_undone_under_a_profile_that_limits_features() {
    let paper = pdfcraft_engine::Session::new().create_from_text("t", "Name: ____________________\n\nPhone: ____________________").unwrap().to_vec();
    let all = pdfcraft_ui_egui::features::FeatureSet::all();
    for profile in [UiProfile::default(), UiProfile { features: Some(all), ..UiProfile::default() }] {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("paper.pdf", None, paper.clone()).unwrap();
        app.set_profile(profile);
        assert!(app.execute("form.detect"));
        let (_, id) = app.active_ids().unwrap();
        assert_eq!(app.session.get(id).unwrap().form.len(), 2, "{:?}", app.toast);
        // The detection is a step the profile's undo log knows, so Undo takes it back.
        app.undo();
        assert!(app.session.get(id).unwrap().form.is_empty(), "undone; {:?}", app.toast);
    }
}

/// A point on page 1 in PDF user space, on screen (`pages`' MediaBox is 200 x 300).
fn at(h: &Harness<'static, PdfCraftApp>, x: f32, y: f32) -> egui::Pos2 {
    let r = h.state().views[0].page_screen_rect(0).expect("page 1 on screen");
    egui::pos2(r.left() + x / 200.0 * r.width(), r.top() + (300.0 - y) / 300.0 * r.height())
}

#[test]
fn dragging_a_mixed_selection_moves_only_the_comments_the_profile_allows() {
    use pdfcraft_engine::{Edit, NewAnnotation, Shape, Style};
    let _gpu = gpu();
    let add = |shape: Shape| {
        Edit::AddAnnotation(NewAnnotation { page: 0, style: Style::default_for(&shape), shape, contents: String::new(), author: "A".into() })
    };
    let mut no_ink = pdfcraft_ui_egui::features::FeatureSet::all();
    no_ink.remove(Feature::Ink);
    for (features, ink_moves) in [(pdfcraft_ui_egui::features::FeatureSet::all(), true), (no_ink, false)] {
        let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
            let mut app = PdfCraftApp::new();
            app.set_option("language", "en").unwrap();
            app.open_bytes("doc.pdf", None, pages(1)).unwrap();
            app.set_option("left", "closed").unwrap();
            app
        });
        h.run_steps(4);
        assert!(h.state_mut().apply_edit(add(Shape::Rectangle { rect: [40.0, 200.0, 100.0, 260.0] })));
        assert!(h.state_mut().apply_edit(add(Shape::Ink { strokes: vec![vec![[120.0, 60.0], [160.0, 100.0]]] })));
        h.state_mut().set_profile(UiProfile { features: Some(features), ..UiProfile::default() });
        let find = |h: &Harness<'static, PdfCraftApp>, subtype: &str| {
            let (_, id) = h.state().active_ids().unwrap();
            let info = &h.state().session.get(id).unwrap().info;
            info.annotations.iter().find(|a| a.subtype == subtype).map(|a| (a.index, a.rect)).unwrap()
        };
        let ((square, square_at), (ink, ink_at)) = (find(&h, "Square"), find(&h, "Ink"));
        h.state_mut().views[0].comments.selected = Some((0, square));
        h.state_mut().views[0].comments.also = vec![(0, ink)];
        h.run_steps(2);
        // Drag the square from its middle.
        let (from, to) = (at(&h, 70.0, 230.0), at(&h, 70.0, 180.0));
        h.hover_at(from);
        h.run_steps(1);
        h.drag_at(from);
        h.run_steps(1);
        for k in 1..=4 {
            h.hover_at(from + (to - from) * (k as f32 / 4.0));
            h.run_steps(1);
        }
        h.drop_at(to);
        h.run_steps(4);
        assert_ne!(find(&h, "Square").1, square_at, "the square moves (Ink allowed: {ink_moves})");
        assert_eq!(find(&h, "Ink").1 != ink_at, ink_moves, "the ink moves only with Ink allowed");
        assert!(h.query_by_label_contains("Not allowed").is_none(), "Ink allowed: {ink_moves}");
    }
}
