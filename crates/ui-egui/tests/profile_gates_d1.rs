//! The UI profile on comments, the Comments panel, Fill & Sign, measurement and stamps: what the
//! profile refuses is hidden (and refused if reached); the default profile changes nothing.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_engine::{Edit, NewAnnotation, Shape, Style};
use pdfcraft_ui_egui::features::Feature;
use pdfcraft_ui_egui::profile::{UiProfile, parts};
use pdfcraft_ui_egui::{PdfCraftApp, QuickTool};

/// See `tests/view.rs`: WARP's shader JIT crashes when two devices compile at once on ARM64.
static GPU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gpu() -> std::sync::MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

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

/// A two-page document; `setup` runs with the default profile, then `profile` is set.
fn harness(profile: UiProfile, setup: impl FnOnce(&mut PdfCraftApp) + 'static) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("doc.pdf", None, fixture(2)).unwrap();
        setup(&mut app);
        app.set_profile(profile);
        app
    });
    h.run_steps(6);
    h
}

/// A rectangle comment, selected, with the Comments panel open.
fn with_comment(profile: UiProfile) -> Harness<'static, PdfCraftApp> {
    harness(profile, |app| {
        assert!(app.apply_edit(rectangle()));
        let (_, id) = app.active_ids().unwrap();
        let index = app.session.get(id).unwrap().info.annotations[0].index;
        app.views[0].comments.selected = Some((0, index));
        app.set_option("panel", "comments").unwrap();
    })
}

/// Every feature but `off`.
fn without(off: &[Feature]) -> UiProfile {
    UiProfile { features: Some(Feature::ALL.into_iter().filter(|f| !off.contains(f)).collect()), ..UiProfile::default() }
}

fn hiding(ids: &[&str]) -> UiProfile {
    UiProfile { hidden: ids.iter().map(|s| s.to_string()).collect(), ..UiProfile::default() }
}

fn count(h: &Harness<'static, PdfCraftApp>, label: &str) -> usize {
    h.query_all_by_label(label).count()
}

fn comments(h: &Harness<'_, PdfCraftApp>) -> usize {
    let (_, id) = h.state().active_ids().unwrap();
    h.state().session.get(id).unwrap().info.annotations.len()
}

/// Open the selected card's "More" menu and count Properties…, Delete, Mark with checkmark and
/// Copy text.
fn card_menu(profile: UiProfile) -> (usize, usize, usize, usize) {
    let mut h = with_comment(profile);
    h.get_by_label("More").click();
    h.run_steps(3);
    (count(&h, "Properties…"), count(&h, "Delete"), count(&h, "Mark with checkmark"), count(&h, "Copy text"))
}

#[test]
fn the_comment_card_menu_keeps_only_what_the_comment_kind_allows() {
    let _gpu = gpu();
    // The checkmark shows on the card and in its menu.
    assert_eq!(card_menu(UiProfile::default()), (1, 1, 2, 1), "the default profile offers everything");
    // Without shapes, a rectangle can't change, but its text can still be copied.
    assert_eq!(card_menu(without(&[Feature::Shapes])), (0, 0, 0, 1));
    // Without copying, Copy text goes.
    assert_eq!(card_menu(hiding(&["edit.copy"])).3, 0);
}

#[test]
fn delete_leaves_a_comment_whose_kind_the_profile_refuses() {
    let _gpu = gpu();
    let mut h = with_comment(UiProfile::default());
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(comments(&h), 0, "the default profile deletes it");

    let mut h = with_comment(without(&[Feature::Shapes]));
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(comments(&h), 1, "the rectangle stays");
    assert!(h.state().views[0].comments.selected.is_some(), "still selected: the key did nothing");
    assert!(h.state().toast.is_none(), "it isn't offered, so nothing is refused either");
}

#[test]
fn the_comments_panel_options_follow_their_commands() {
    let _gpu = gpu();
    let options = |profile: UiProfile| {
        let mut h = with_comment(profile);
        h.get_by_label("More options").click();
        h.run_steps(3);
        (count(&h, "Import comments…"), count(&h, "Export all to data file…"))
    };
    assert_eq!(options(UiProfile::default()), (1, 1));
    assert_eq!(options(hiding(&["comment.import"])), (0, 1));
}

#[test]
fn attachments_are_saved_or_opened_only_when_the_profile_allows_it() {
    let _gpu = gpu();
    let mut h = harness(UiProfile::default(), |_| {});
    let (_, id) = h.state().active_ids().unwrap();
    h.state_mut().attachment_action(id, 0, false);
    assert!(h.state().toast.is_none(), "no such file: nothing to say");
    for (profile, open) in [(without(&[Feature::Download]), false), (hiding(&["file.open"]), true)] {
        let mut h = harness(profile, |_| {});
        h.state_mut().attachment_action(id, 0, open);
        assert_eq!(h.state().toast.clone().map(|t| t.0).as_deref(), Some("Not allowed"));
    }
}

#[test]
fn the_measuring_panel_hides_refused_tools_and_arms_the_others_directly() {
    let _gpu = gpu();
    let panel = |profile: UiProfile| {
        harness(profile, |app| {
            app.set_option("left", "open").unwrap();
            app.set_option("tool", "measure").unwrap();
        })
    };
    let mut h = panel(UiProfile::default());
    assert_eq!((count(&h, "Distance"), count(&h, "Apply scale"), count(&h, "Export measurements as CSV")), (1, 1, 1));
    h.get_by_label("Distance").click();
    h.run_steps(3);
    assert!(matches!(h.state().quick_tool, QuickTool::Measure(_)), "the default profile arms it at once");

    let h = panel(hiding(&["measure.distance", "measure.scale", "measure.export"]));
    assert_eq!((count(&h, "Distance"), count(&h, "Apply scale"), count(&h, "Export measurements as CSV")), (0, 0, 0));
    assert_eq!(count(&h, "Area"), 1, "the other tools stay");
}

#[test]
fn custom_stamps_are_managed_only_while_the_profile_allows_it() {
    let _gpu = gpu();
    let create = |profile: UiProfile| {
        let h = harness(profile, |app| {
            app.set_option("left", "open").unwrap();
            app.set_option("tool", "stamp").unwrap();
        });
        count(&h, "Create custom stamp…")
    };
    assert_eq!(create(UiProfile::default()), 1);
    let mut kept = UiProfile::default();
    kept.hidden.insert(parts::STAMPS_CUSTOM.to_string());
    assert_eq!(create(kept), 0);
}

#[test]
fn fill_and_sign_leaves_out_refused_signature_entries() {
    let _gpu = gpu();
    let entries = |profile: UiProfile| {
        let h = harness(profile, |app| {
            app.set_option("left", "open").unwrap();
            app.set_option("tool", "fill_sign").unwrap();
        });
        (count(&h, "Add signature"), count(&h, "Add initials"))
    };
    assert_eq!(entries(UiProfile::default()), (1, 1));
    assert_eq!(entries(hiding(&["sign.fill.initials"])), (1, 0));
}

#[test]
fn delete_leaves_a_locked_comment_with_the_default_profile_too() {
    // Deliberate: the menus already greyed Delete for a locked comment; the key now agrees.
    let _gpu = gpu();
    let mut h = with_comment(UiProfile::default());
    let (page, index) = h.state().views[0].comments.selected.unwrap();
    assert!(h.state_mut().apply_edit(Edit::LockAnnotation { page, index, locked: true }));
    h.state_mut().views[0].comments.selected = Some((page, index));
    h.run_steps(2);
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(comments(&h), 1, "a locked comment stays");
}

/// One 200×300 page with a stroked path from (40, 40) to (160, 40) to (160, 200).
fn drawing() -> Vec<u8> {
    let content = "40 40 m 160 40 l 160 200 l S";
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 300] >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
    ];
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

#[test]
fn measuring_snaps_to_the_drawing_only_while_snapping_is_allowed() {
    let _gpu = gpu();
    // The first point of a distance measured just off the path's corner (40, 40).
    let first_point = |profile: UiProfile| {
        let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
            let mut app = PdfCraftApp::new();
            app.set_option("language", "en").unwrap();
            app.open_bytes("drawing.pdf", None, drawing()).unwrap();
            app.set_option("left", "closed").unwrap();
            app.set_option("panel", "none").unwrap();
            app.set_option("zoom", "100").unwrap();
            app.set_profile(profile);
            app.set_option("quick", "measure-distance").unwrap();
            app
        });
        h.run_steps(6);
        let r = h.state().views[0].page_screen_rect(0).expect("page 1 on screen");
        let at = egui::pos2(r.min.x + r.width() * 41.0 / 200.0, r.min.y + r.height() * (1.0 - 41.0 / 300.0));
        h.hover_at(at);
        h.run_steps(2);
        h.drag_at(at);
        h.run_steps(1);
        h.drop_at(at);
        h.run_steps(3);
        let p = *h.state().views[0].measure.points.first().expect("a point measured");
        ((p[0] - 40.0).powi(2) + (p[1] - 40.0).powi(2)).sqrt()
    };
    assert!(first_point(UiProfile::default()) < 1e-6, "snapped to the corner");
    assert!(first_point(hiding(&["measure.snap"])) > 0.5, "where the pointer is");
}

#[test]
fn changing_the_saved_signature_follows_the_legacy_flags() {
    let commands = ["sign.fill.signature.change", "sign.fill.signature.remove", "sign.fill.initials.change", "sign.fill.initials.remove"];
    let read_only = UiProfile { read_only: true, ..UiProfile::default() };
    for id in commands {
        assert!(UiProfile::default().allows_id(id), "{id}");
        assert!(UiProfile { comments_only: true, ..UiProfile::default() }.allows_id(id), "{id}: Fill & Sign is a comment feature");
        assert!(!read_only.allows_id(id), "{id} in a view-only profile");
    }
    // The Signature dialog has nothing left to offer in a view-only profile.
    let _gpu = gpu();
    let mut h = harness(UiProfile::default(), |_| {});
    h.state_mut().dialog = Some(pdfcraft_ui_egui::Dialog::Signature);
    h.state_mut().set_profile(read_only);
    assert_eq!(h.state().dialog, None);
}
