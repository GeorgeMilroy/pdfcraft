//! The UI profile (crate::profile): a locked view-only mode, comments-only editing, and hidden
//! commands and tool groups, enforced where things happen; the default profile changes nothing.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_engine::{Edit, NewAnnotation, Shape, Style};
use pdfcraft_ui_egui::profile::{UiProfile, parts};
use pdfcraft_ui_egui::{Mode, PdfCraftApp};

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

/// Whether a workspace mode tab (a button) labelled `label` shows.
fn mode_tab(h: &Harness<'_, PdfCraftApp>, label: &str) -> bool {
    h.query_all_by(|n| n.role() == egui::accesskit::Role::Button && n.label().as_deref() == Some(label)).next().is_some()
}

fn dirty(h: &Harness<'_, PdfCraftApp>) -> bool {
    h.state().first_dirty().is_some()
}

fn comments(h: &Harness<'_, PdfCraftApp>) -> usize {
    let (_, id) = h.state().active_ids().unwrap();
    h.state().session.get(id).unwrap().info.annotations.len()
}

#[test]
fn the_default_profile_changes_nothing() {
    let mut h = harness(UiProfile::default());
    assert!(h.state_mut().apply_edit(rectangle()));
    assert_eq!(comments(&h), 1);
    assert!(h.state_mut().execute("comment.note"));
    assert!(h.state_mut().execute("page.rotate_cw") || pdfcraft_engine::commands::command("page.rotate_cw").is_none());
    h.state_mut().set_option("mode", "edit").unwrap();
    h.run_steps(2);
    assert_eq!(h.state().mode, Mode::Edit);
}

#[test]
fn view_only_refuses_every_change_and_stays_in_read() {
    let mut h = harness(UiProfile { read_only: true, ..UiProfile::default() });
    assert_eq!(h.state().mode, Mode::Read, "pinned to Read");
    // Edits, whatever asks for them.
    assert!(!h.state_mut().apply_edit(rectangle()));
    assert_eq!(comments(&h), 0);
    assert!(!dirty(&h));
    // Editing commands, from menus, shortcuts or the palette alike.
    for id in ["comment.note", "comment.highlight", "edit.undo", "file.save", "comment.flatten", "sign.fill.text"] {
        assert!(!h.state_mut().execute(id), "{id} ran in view-only");
    }
    // Viewing commands still work.
    assert!(h.state_mut().execute("view.zoom_in") || pdfcraft_engine::commands::command("view.zoom_in").is_none());
    // Switching workspaces doesn't stick, and no drawing tool stays selected.
    h.state_mut().set_option("mode", "all").unwrap();
    h.state_mut().quick_tool = pdfcraft_ui_egui::QuickTool::Comment(pdfcraft_ui_egui::comments::CommentTool::Note);
    h.run_steps(2);
    assert_eq!(h.state().mode, Mode::Read);
    assert_eq!(h.state().quick_tool, pdfcraft_ui_egui::QuickTool::Select);
    // Save refuses even when asked directly.
    assert!(!h.state_mut().save_view(0, pdfcraft_ui_egui::SaveTarget::InPlace));
    // No mode tabs to leave Read with (the menu bar's Edit menu is not a mode tab).
    assert!(!mode_tab(&h, "Edit") && !mode_tab(&h, "All tools"));
}

#[test]
fn view_only_refuses_undo_of_earlier_changes() {
    let mut h = harness(UiProfile::default());
    assert!(h.state_mut().apply_edit(rectangle()));
    h.state_mut().profile.read_only = true;
    h.state_mut().undo();
    h.run_steps(2);
    assert_eq!(comments(&h), 1, "undo is a change too");
}

#[test]
fn comments_only_allows_comments_and_refuses_the_rest() {
    let mut h = harness(UiProfile { comments_only: true, ..UiProfile::default() });
    assert!(h.state_mut().apply_edit(rectangle()));
    assert_eq!(comments(&h), 1);
    assert!(!h.state_mut().apply_edit(Edit::DeletePages { pages: vec![1] }), "page changes are refused");
    assert!(!h.state_mut().apply_edit(Edit::SetInfo { key: "Title".into(), value: "x".into() }));
    let (_, id) = h.state().active_ids().unwrap();
    assert_eq!(h.state().session.get(id).unwrap().info.pages.len(), 2);
    // A batch is allowed only when every edit in it is.
    assert!(!h.state_mut().apply_edit(Edit::Batch { label: "mixed".into(), edits: vec![rectangle(), Edit::DeletePages { pages: vec![1] }] }));
    assert_eq!(comments(&h), 1);
    // Undo of a comment still works.
    h.state_mut().undo();
    assert_eq!(comments(&h), 0);
}

#[test]
fn hidden_commands_and_groups_do_not_show_or_run() {
    let mut profile = UiProfile::default();
    for id in ["print.dialog", "file.save", "comment.highlight", "group:export", parts::MODE_TABS, parts::TABS] {
        profile.hidden.insert(id.into());
    }
    let mut h = harness(profile);
    assert!(!h.state_mut().execute("print.dialog"));
    assert!(!h.state_mut().execute("file.save"));
    assert!(!h.state_mut().execute("comment.highlight"));
    assert!(h.state_mut().execute("comment.underline"), "other tools still run");
    // The All tools list leaves out the hidden group; the tab strip and mode tabs are gone.
    h.state_mut().set_option("mode", "all").unwrap();
    h.state_mut().set_option("left", "all").ok();
    h.run_steps(4);
    assert!(h.query_by_label("Export a PDF").is_none(), "hidden tool group listed");
    assert!(h.query_by_label("Add comments").is_some(), "other tool groups listed");
    assert!(h.query_by_label("Convert").is_none(), "mode tabs hidden");
    assert!(h.query_by_label("Open").is_none(), "tab strip hidden");
}

#[test]
fn all_tools_lists_every_group_the_profile_keeps_without_view_more() {
    // Keep four groups, one of them past the first 14 of the catalogue (Measure objects).
    let keep = ["fill_sign", "comment", "stamp", "measure"];
    let mut profile = UiProfile::default();
    for g in pdfcraft_engine::catalog::TOOL_GROUPS {
        if !keep.contains(&g.id) {
            profile.hidden.insert(format!("group:{}", g.id));
        }
    }
    let mut h = harness(profile);
    h.state_mut().set_option("mode", "all").unwrap();
    h.run_steps(4);
    for label in ["Fill & Sign", "Add comments", "Add a stamp", "Measure objects"] {
        assert!(h.query_all_by_label(label).count() >= 1, "{label} missing from All tools");
    }
    assert!(h.query_by_label("View more").is_none(), "nothing left to show");
}
