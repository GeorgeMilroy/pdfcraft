//! What a UI profile refuses is hidden in the window chrome, menus, palette, Home, the rails and
//! the panel layout, and refused when reached anyway; the default profile shows all of it.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::features::Feature;
use pdfcraft_ui_egui::profile::{UiProfile, parts};
use pdfcraft_ui_egui::{Dialog, LeftPanel, Mode, PdfCraftApp, QuickTool, RightPanel};

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

fn harness_with(profile: UiProfile, document: bool) -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        if document {
            app.open_bytes("doc.pdf", None, fixture(2)).unwrap();
        } else {
            // Home on its own, without the Tools panel beside it.
            app.set_option("mode", "read").unwrap();
        }
        app.set_profile(profile);
        app
    });
    h.run_steps(4);
    h
}

fn harness(profile: UiProfile) -> Harness<'static, PdfCraftApp> {
    harness_with(profile, true)
}

/// Comments and markup only: no app features (opening files, tabs, combining), no
/// preferences, help, palette, search or accessibility.
fn comments_only() -> UiProfile {
    UiProfile { features: Some([Feature::Comments, Feature::Markup].into_iter().collect()), ..UiProfile::default() }
}

fn shown(h: &Harness<'_, PdfCraftApp>, label: &str) -> bool {
    h.query_all_by_label(label).next().is_some()
}

#[test]
fn the_tab_strip_and_mode_bar_hide_what_the_profile_refuses() {
    let all = ["Open", "Home", "Keyboard shortcuts", "Edit", "Convert", "E-Sign", "Find tools and commands"];
    let h = harness(UiProfile::default());
    for label in all {
        assert!(shown(&h, label), "{label} missing with the default profile");
    }
    let h = harness(comments_only());
    for label in ["Open", "Keyboard shortcuts", "Edit", "Convert", "E-Sign", "Find tools and commands"] {
        assert!(!shown(&h, label), "{label} drawn although the profile refuses it");
    }
    assert!(shown(&h, "Read") && shown(&h, "All tools"), "the workspaces that stay");
    // Home is its own part.
    let h = harness(UiProfile { hidden: [parts::HOME.to_string()].into(), ..UiProfile::default() });
    assert!(!shown(&h, "Home"));
}

#[test]
fn the_palette_does_not_open_when_refused() {
    let mut h = harness(UiProfile { hidden: [parts::SEARCH.to_string()].into(), ..UiProfile::default() });
    assert!(!h.state_mut().execute("view.palette"), "ui:search refuses the palette command too");
    assert!(h.state_mut().set_option("palette", "rotate").is_err());
    h.state_mut().palette_open = true;
    h.run_steps(2);
    assert!(!h.state().palette_open, "an open palette is put down");
    let mut h = harness(UiProfile::default());
    assert!(h.state_mut().set_option("palette", "rotate").is_ok());
    h.run_steps(2);
    assert!(h.state().palette_open);
}

#[test]
fn the_rail_shows_only_the_panels_and_parts_the_profile_keeps() {
    let h = harness(UiProfile::default());
    for label in ["Comments", "Signatures", "Tags", "Toggle fit page / fit width", "Next page"] {
        assert!(shown(&h, label), "{label} missing from the default rail");
    }
    let mut h = harness(comments_only());
    assert!(shown(&h, "Comments"), "a viewer panel stays");
    assert!(!shown(&h, "Signatures") && !shown(&h, "Tags"), "panels whose feature is off");
    // The single way in to a panel refuses the ones the profile hides.
    h.state_mut().choose_right_panel(Some(RightPanel::Tags));
    assert_ne!(h.state().right, Some(RightPanel::Tags));
    assert!(h.state_mut().set_option("panel", "signatures").is_err());

    let hidden = [parts::RAIL_ZOOM.to_string(), parts::RAIL_PAGE_NAV.to_string()].into();
    let h = harness(UiProfile { hidden, ..UiProfile::default() });
    assert!(shown(&h, "Comments"));
    assert!(!shown(&h, "Toggle fit page / fit width") && !shown(&h, "Next page"), "rail parts hidden");
    let h = harness(UiProfile { hidden: [parts::RAIL.to_string()].into(), ..UiProfile::default() });
    assert!(!shown(&h, "Bookmarks"), "the whole rail hidden");
}

#[test]
fn changing_the_profile_puts_down_what_it_refuses() {
    let mut h = harness(UiProfile::default());
    let app = h.state_mut();
    app.quick_tool = QuickTool::EditText;
    app.dialog = Some(Dialog::Preferences);
    app.right = Some(RightPanel::Tags);
    app.left = LeftPanel::Tool("form");
    app.left_open = true;
    assert!(app.is_preparing());
    app.views[0].organize = true;
    app.palette_open = true;
    app.set_profile(UiProfile { comments_only: true, ..comments_only() });
    assert_eq!(app.quick_tool, QuickTool::Select);
    assert_eq!(app.dialog, None);
    assert_eq!(app.right, None);
    assert_eq!(app.left, LeftPanel::AllTools);
    assert!(!app.is_preparing());
    assert!(!app.views[0].organize);
    assert!(!app.palette_open);
    // Nor can they be picked again.
    assert!(app.set_option("quick", "edit-text").is_err());
    assert!(app.set_option("dialog", "preferences").is_err());
    assert!(app.set_option("tool", "form").is_err());
    assert!(app.set_option("organize", "on").is_err());
    assert!(app.set_option("quick", "highlight").is_ok(), "a comment tool is still allowed");
    app.open_combine_tab();
    assert!(!app.combine_tab.open, "Combine files is an app feature");
}

#[test]
fn form_preparation_needs_form_design() {
    let mut h = harness(UiProfile { comments_only: true, ..UiProfile::default() });
    let app = h.state_mut();
    app.left = LeftPanel::Tool("form");
    app.left_open = true;
    assert!(!app.is_preparing());
    let mut h = harness(UiProfile::default());
    let app = h.state_mut();
    app.left = LeftPanel::Tool("form");
    app.left_open = true;
    assert!(app.is_preparing());
}

#[test]
fn workspaces_whose_tools_the_profile_refuses_have_no_tab() {
    // Comments only, by permission rather than by feature: editing content stays refused, so
    // Edit (which arms Edit text & images) has nothing to show.
    let h = harness(UiProfile { comments_only: true, ..UiProfile::default() });
    assert!(!shown(&h, "Edit"));
    assert!(shown(&h, "Convert") && shown(&h, "E-Sign"), "exporting copies, and Fill & Sign marks are comments");
    let mut h = harness(UiProfile::default());
    h.get_by_label("Edit").click();
    h.run_steps(2);
    assert_eq!(h.state().mode, Mode::Edit);
    assert_eq!(h.state().quick_tool, QuickTool::EditText);
}

#[test]
fn a_hidden_tools_panel_and_its_rail_stay_hidden() {
    let mut h = harness(UiProfile { tools_rail: true, hidden: [parts::TOOLS_PANEL.to_string()].into(), ..UiProfile::default() });
    h.state_mut().set_option("mode", "all").unwrap();
    h.run_steps(4);
    assert!(!shown(&h, "Add comments"), "the Tools panel is hidden");
    assert!(!shown(&h, "Tools"), "and no rail brings it back");
    let mut h = harness(UiProfile::default());
    h.state_mut().set_option("mode", "all").unwrap();
    h.run_steps(4);
    assert!(shown(&h, "Add comments"));
}

#[test]
fn a_locked_layout_refuses_moving_and_closing_panels() {
    let mut h = harness(UiProfile { lock_layout: true, ..UiProfile::default() });
    let app = h.state_mut();
    let close = serde_json::json!({"operation": "close", "panel": "tools"});
    assert!(pdfcraft_ui_egui::docking::command(app, false, &close).is_err());
    let float = serde_json::json!({"operation": "float", "panel": "tools", "rect": [10, 10, 300, 300]});
    assert!(pdfcraft_ui_egui::docking::command(app, false, &float).is_err());
    assert!(pdfcraft_ui_egui::docking::command(app, true, &serde_json::Value::Null).is_err(), "nor reset");
    let activate = serde_json::json!({"operation": "activate", "panel": "tools"});
    assert!(pdfcraft_ui_egui::docking::command(app, false, &activate).is_ok());
    let mut h = harness(UiProfile::default());
    assert!(pdfcraft_ui_egui::docking::command(h.state_mut(), false, &close).is_ok());
}

#[test]
fn home_offers_only_what_the_profile_allows() {
    let h = harness_with(UiProfile::default(), false);
    for label in ["Open file", "Add comments", "Organize pages", "Recent"] {
        assert!(shown(&h, label), "{label} missing from the default Home");
    }
    let h = harness_with(comments_only(), false);
    assert!(shown(&h, "Add comments"));
    for label in ["Open file", "Organize pages", "Recent", "Protect a PDF"] {
        assert!(!shown(&h, label), "{label} on Home although the profile refuses it");
    }
}

#[test]
fn menus_leave_out_what_the_profile_refuses() {
    let mut h = harness(UiProfile::default());
    h.get_by_label("Menu").click();
    h.run_steps(2);
    for label in ["File ⏵", "Pages ⏵", "Help ⏵"] {
        assert!(shown(&h, label), "{label} menu missing with the default profile");
    }
    let mut h = harness(UiProfile { comments_only: true, ..comments_only() });
    h.get_by_label("Menu").click();
    h.run_steps(2);
    assert!(shown(&h, "View ⏵"), "View keeps the zoom and page display");
    assert!(!shown(&h, "Help ⏵") && !shown(&h, "Pages ⏵"), "menus with nothing left in them");
}

fn hiding(ids: &[&str]) -> UiProfile {
    UiProfile { hidden: ids.iter().map(|s| s.to_string()).collect(), ..UiProfile::default() }
}

#[test]
fn a_dialog_stays_open_while_any_command_that_opens_it_is_allowed() {
    use pdfcraft_ui_egui::PropsTab;
    // Properties: either command opens it, on a tab of a command allowed (Security is
    // protect.properties, the others file.properties).
    for hidden in ["protect.properties", "file.properties"] {
        let mut h = harness(hiding(&[hidden]));
        let kept = if hidden == "file.properties" { PropsTab::Security } else { PropsTab::Description };
        for tab in [PropsTab::Description, PropsTab::Security] {
            h.state_mut().dialog = Some(Dialog::Properties(tab));
            h.run_steps(2);
            assert_eq!(h.state().dialog, Some(Dialog::Properties(kept)), "{tab:?} with {hidden} hidden");
        }
    }
    let mut h = harness(hiding(&["file.properties", "protect.properties"]));
    h.state_mut().dialog = Some(Dialog::Properties(PropsTab::Security));
    h.run_steps(2);
    assert_eq!(h.state().dialog, None, "closed when neither is allowed");
    // Sign: digital signing or certifying; Signature: the signature or the initials.
    // (Set before the profile, which puts down what it refuses at once.)
    for (hidden, dialog) in [("sign.digital", Dialog::Sign), ("sign.fill.signature", Dialog::Signature)] {
        let mut h = harness(UiProfile::default());
        h.state_mut().dialog = Some(dialog);
        h.state_mut().set_profile(hiding(&[hidden]));
        assert_eq!(h.state().dialog, Some(dialog), "{dialog:?} closed with only {hidden} hidden");
        let all: &[&str] = if dialog == Dialog::Sign {
            &["sign.digital", "sign.certify"]
        } else {
            &["sign.fill.signature", "sign.fill.initials", "sign.fill.signature.change", "sign.fill.initials.change"]
        };
        h.state_mut().set_profile(hiding(all));
        assert_eq!(h.state().dialog, None, "{dialog:?} closes when none of its commands is allowed");
    }
}

#[test]
fn a_refused_dialog_request_leaves_the_open_dialog_alone() {
    let mut h = harness(hiding(&["page.split"]));
    h.state_mut().set_option("dialog", "about").unwrap();
    assert!(h.state_mut().set_option("dialog", "split").is_err());
    assert_eq!(h.state().dialog, Some(Dialog::About), "the refusal changed nothing");
    // Certifying is its own command.
    let mut h = harness(hiding(&["sign.certify"]));
    assert!(h.state_mut().set_option("dialog", "certify").is_err());
    assert_eq!(h.state().dialog, None);
}

#[test]
fn a_workspace_whose_tools_the_profile_hides_is_left() {
    let mut h = harness(UiProfile::default());
    h.state_mut().set_option("mode", "edit").unwrap();
    h.run_steps(2);
    assert_eq!(h.state().mode, Mode::Edit, "the default profile keeps it");
    h.state_mut().set_profile(hiding(&["group:edit"]));
    assert_eq!(h.state().mode, Mode::AllTools, "a profile change leaves it");
    assert!(h.state_mut().set_option("mode", "edit").is_err(), "and it can't be chosen again");
    assert_eq!(h.state().mode, Mode::AllTools);
    let mut h = harness(UiProfile { comments_only: true, ..UiProfile::default() });
    assert!(h.state_mut().set_option("mode", "edit").is_err());
    assert!(h.state_mut().set_option("mode", "sign").is_ok(), "Fill & Sign marks are comments");
}

#[test]
fn recent_files_on_home_need_file_open() {
    let recent = |profile: UiProfile| {
        let mut h = harness_with(profile, false);
        h.state_mut().set_option("home", "").unwrap();
        h.run_steps(3);
        shown(&h, "Recent")
    };
    assert!(recent(UiProfile::default()));
    assert!(!recent(hiding(&["file.open"])));
}

#[test]
fn the_theme_button_cycles_to_the_next_allowed_theme() {
    let mut h = harness(hiding(&["view.theme.dark"]));
    h.state_mut().set_option("theme", "light").unwrap();
    h.run_steps(2);
    // Dark is refused, System and Light aren't: the button shows and skips Dark.
    h.get_by_label("Display theme: Light gray").click();
    h.run_steps(2);
    assert_eq!(h.state().theme_preference, pdfcraft_ui_egui::theme::ThemePreference::System);
}
