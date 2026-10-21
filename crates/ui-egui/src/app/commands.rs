use super::*;

impl PdfCraftApp {
    /// Run a catalogue command. Commands that aren't implemented yet say which milestone ships them.
    pub fn run_command(&mut self, command: &str) {
        if let Some(spec) = pdfcraft_engine::commands::command(command) {
            use pdfcraft_engine::commands::Needs;
            // A tool that needs a document, chosen with none open, asks for the file and then
            // runs on it, instead of only saying to open one first.
            let needs_document = !matches!(spec.needs, Needs::Nothing | Needs::Undo | Needs::Redo);
            let is_tool = pdfcraft_engine::catalog::TOOL_GROUPS
                .iter()
                .flat_map(|g| g.sections.iter().flat_map(|s| s.items.iter()))
                .any(|i| i.command == spec.id);
            if self.active.is_none() && needs_document && is_tool && !self.command_enabled(spec) {
                self.open_then_run(spec);
                return;
            }
            self.execute(command);
            return;
        }
        // Not implemented yet: say which milestone ships it.
        let when = pdfcraft_engine::catalog::TOOL_GROUPS
            .iter()
            .flat_map(|g| g.sections.iter().flat_map(|s| s.items.iter()))
            .find(|i| i.command == command)
            .map(|i| match i.availability {
                pdfcraft_engine::catalog::Availability::Planned(m) => crate::i18n::fmt(tl!("ships in milestone {m}"), &[("m", m)]),
                pdfcraft_engine::catalog::Availability::Provider => tl!("needs an AI provider (off by default)").to_string(),
                pdfcraft_engine::catalog::Availability::Ready => tl!("is available").to_string(),
            })
            .unwrap_or_else(|| tl!("is not available yet").to_string());
        self.notify_fmt("`{command}` {when}", &[("command", command), ("when", &when)]);
    }

    /// Ask for a file to open, then run `spec` on it once it is open (or unlocked). Nothing runs
    /// when the picker is cancelled or the file doesn't open.
    #[cfg(not(target_arch = "wasm32"))]
    fn open_then_run(&mut self, spec: &'static pdfcraft_engine::commands::CommandSpec) {
        self.startup_superseded = true;
        let dialog = Self::open_picker().set_title(tl!(spec.label).trim_end_matches('…'));
        let command = spec.id;
        self.ask_one(pickers::Ask::File(dialog), None, move |app, path| {
            let path = path.to_string_lossy().into_owned();
            app.open_path_then(&path, Some(command));
        });
    }

    /// Browsers read the picked file asynchronously; it opens, and the tool can be chosen again.
    #[cfg(target_arch = "wasm32")]
    fn open_then_run(&mut self, _spec: &'static pdfcraft_engine::commands::CommandSpec) {
        self.open_dialog();
    }

    pub(crate) fn reveal_tools(&mut self) {
        self.left_open = true;
    }

    /// Uses `profile` from now on and puts down what it no longer allows: an armed tool, an open
    /// panel or dialog. Hosts set the profile this way rather than assigning the field.
    pub fn set_profile(&mut self, profile: crate::profile::UiProfile) {
        self.profile = profile;
        self.enforce_profile();
        // A host may set it between `logic` and `ui`: this frame's panels draw with it.
        if let Some(ctx) = self.ctx.clone() {
            dialogs::publish_profile(self, &ctx);
        }
    }

    /// Puts down what the profile doesn't allow (after it changed; checked every frame too): an
    /// armed tool, an open panel, tool group or dialog, the find bar, the page grid, form
    /// preparation and the Combine files tab.
    pub(crate) fn enforce_profile(&mut self) {
        if self.right.is_some_and(|p| !self.profile.shows_panel(p)) {
            self.choose_right_panel(None);
        }
        if self.quick_tool.command().is_some_and(|id| !self.profile.allows_id(id)) {
            if matches!(self.quick_tool, QuickTool::EditText | QuickTool::AddText) {
                self.put_down_edit_text();
            } else {
                self.quick_tool = QuickTool::Select;
            }
        }
        if let LeftPanel::Tool(id) = self.left
            && !self.shows_tool_group(id)
        {
            self.left = LeftPanel::AllTools;
        }
        if self.form_preview && !self.profile.allows_feature(features::Feature::FormDesign) {
            self.form_preview = false;
        }
        if self.dialog.is_some_and(|d| !self.allows_dialog(d)) {
            self.dialog = None;
        }
        // Signing and certifying are separate commands: the one the draft is for must be allowed.
        if self.dialog == Some(Dialog::Sign)
            && let Some(d) = &self.sign_draft
            && !self.profile.allows_id(if d.certify.is_some() { "sign.certify" } else { "sign.digital" })
        {
            self.dialog = None;
        }
        // A Properties tab the profile refuses gives way to one it allows (the dialog is open, so
        // one is).
        if let Some(Dialog::Properties(tab)) = self.dialog
            && !self.profile.allows_id(tab.command())
        {
            self.dialog = PropsTab::ALL.into_iter().find(|t| self.profile.allows_id(t.command())).map(Dialog::Properties);
        }
        if self.dialog == Some(Dialog::CreateStamp) && !self.manages_custom_stamps() {
            self.dialog = None;
        }
        if !self.allows_mode(self.mode) {
            self.switch_mode(Mode::AllTools, false);
        }
        // Without JavaScript, documents' scripts (open actions, calculations) don't run at all;
        // the user's preference comes back with the feature.
        if !self.profile.allows_feature(features::Feature::JavaScript) {
            if self.js_held.is_none() {
                self.js_held = Some(self.session.javascript());
                self.session.set_javascript(false);
            }
        } else if let Some(on) = self.js_held.take() {
            self.session.set_javascript(on);
        }
        let (find, organize) = (self.profile.allows_id("edit.find"), self.shows_command("page.organize"));
        for view in &mut self.views {
            if !find {
                view.find = None;
            }
            if !organize {
                view.organize = false;
            }
        }
        if self.palette_open && !self.shows_command("view.palette") {
            self.palette_open = false;
        }
        if self.combine_tab.open && !self.profile.allows_id("page.combine") {
            self.close_combine_tab();
        }
    }

    /// Whether a command shows and runs in this profile: [`profile::UiProfile::allows_id`], and
    /// what only the app knows. Read mode can't be left in a view-only profile, the palette
    /// belongs to the search box, and the page grid is for changing pages.
    pub(crate) fn shows_command(&self, id: &str) -> bool {
        self.profile.allows_id(id)
            && match id {
                "view.read_mode" => !self.profile.read_only,
                "view.palette" => self.profile.shows_part(profile::parts::SEARCH),
                "page.organize" => self.profile.allows_feature(features::Feature::Pages),
                "ui.dock.reset" => !self.profile.lock_layout,
                "view.tools_pane" => self.profile.shows_part(profile::parts::TOOLS_PANEL),
                _ => true,
            }
    }

    /// Whether the profile allows one of the commands that open dialog `d` ([`Dialog::commands`]).
    pub(crate) fn allows_dialog(&self, d: Dialog) -> bool {
        let ids = d.commands();
        ids.is_empty() || ids.iter().any(|id| self.profile.allows_id(id))
    }

    /// Whether the workspace shows: one with a tool group of its own (Edit, Convert, E-Sign)
    /// needs that group shown.
    pub(crate) fn allows_mode(&self, mode: Mode) -> bool {
        mode.tool_group().is_none_or(|id| self.shows_tool_group(id))
    }

    /// Whether the tool group `id` shows: [`profile::UiProfile::shows_group`], and the feature the
    /// group serves is on (a group can keep entries that only copy, such as Extract pages, after
    /// its feature is gone): the same test as the All tools list ([`panels::group_shown`]).
    pub(crate) fn shows_tool_group(&self, id: &str) -> bool {
        pdfcraft_engine::catalog::group(id).is_some_and(|g| panels::group_shown(&self.profile, g))
    }

    /// Enter a tool from the catalogue, Home, palette or control channel. Organize is a
    /// workspace, so opening its panel also opens the page grid (without toggling it off).
    /// False when the profile doesn't show the group.
    pub(crate) fn select_tool(&mut self, id: &'static str) -> bool {
        if !self.shows_tool_group(id) {
            return false;
        }
        // The tool's panel shows, not the expanded All tools view over it.
        self.all_tools_full = false;
        self.left = LeftPanel::Tool(id);
        self.reveal_tools();
        let organize = id == "organize" && self.shows_command("page.organize");
        if let Some(view) = self.active.and_then(|i| self.views.get_mut(i)) {
            if organize {
                view.open_organize();
            } else {
                view.organize = false;
            }
        }
        true
    }

    /// Select the workspace and its matching tool panel, just like the mode bar. The tool goes
    /// with the workspace as [`Self::enter_mode`] sets it.
    pub(crate) fn select_mode(&mut self, mode: Mode) {
        // A workspace whose tool group the profile hides is All tools.
        let mode = if self.allows_mode(mode) { mode } else { Mode::AllTools };
        self.enter_mode(mode);
        self.reveal_tools();
        if let Some(view) = self.active.and_then(|i| self.views.get_mut(i)) {
            view.organize = false;
        }
        self.left = match mode.tool_group() {
            Some(id) if self.shows_tool_group(id) => LeftPanel::Tool(id),
            _ => LeftPanel::AllTools,
        };
    }

    /// Changes the workspace and the tool that goes with it. The mode bar (through
    /// [`Self::select_mode`]), the Read mode command and the tool panels' back arrows all come
    /// here, so they leave Edit alike. Edit turns on Edit text & images at once, as Acrobat does:
    /// every paragraph and image on the page gets its box and is editable with one click. Leaving
    /// Edit puts the editing tools down again and applies a paragraph still open in the editor.
    pub(crate) fn enter_mode(&mut self, mode: Mode) {
        self.mode = mode;
        // Another workspace puts the expanded All tools view away.
        if mode != Mode::AllTools {
            self.all_tools_full = false;
        }
        if mode == Mode::Edit && self.profile.allows_id("edit.edit_text") {
            self.quick_tool = QuickTool::EditText;
        } else if matches!(self.quick_tool, QuickTool::EditText | QuickTool::AddText) {
            self.put_down_edit_text();
        }
        // Read mode hides the quick toolbar, so no other tool may stay active there. Opening a
        // document with Read as the default mode comes here without `switch_mode`.
        if mode == Mode::Read {
            self.quick_tool = QuickTool::Select;
        }
    }

    /// Fill the workspace with the expanded All tools view, entering All tools the way the mode
    /// bar does: leaving Read restores its right panel and leaving Edit puts the editing tool down.
    pub(crate) fn expand_all_tools(&mut self) {
        if self.mode != Mode::AllTools {
            self.switch_mode(Mode::AllTools, true);
        }
        self.all_tools_full = true;
        self.all_tools_tab = self.shown_tab();
    }

    /// The tab at the front: the active document, or else whether Combine files shows over Home.
    pub(super) fn shown_tab(&self) -> (Option<DocId>, bool) {
        (self.active.and_then(|i| self.views.get(i)).map(|v| v.id), self.combine_showing())
    }

    /// Back to the Select tool from an editing tool. A paragraph still open in the editor is
    /// applied, as clicking away from it does; left open, its page would keep showing the
    /// paragraph's text removed with no editor over it.
    pub(crate) fn put_down_edit_text(&mut self) {
        self.quick_tool = QuickTool::Select;
        if let Some(ed) = self.active.and_then(|i| self.views.get_mut(i)).and_then(|v| v.line_editor.take())
            && let Some(edit) = edit_text_ui::finished_edit(ed)
        {
            self.apply_edit(edit);
        }
    }

    /// Switches workspace because the user chose to (mode bar, Read mode command). Entering Read
    /// mode closes the right panel, as Acrobat's Read mode hides its panes, and leaving it reopens
    /// that panel. The rail stays in Read mode, and its buttons open panels there too (#932).
    /// Read mode hides the quick toolbar, so entering it also puts the current tool down.
    pub(crate) fn switch_mode(&mut self, mode: Mode, select_tools: bool) {
        let mode = if self.allows_mode(mode) { mode } else { Mode::AllTools };
        let was_read = self.mode == Mode::Read;
        if select_tools {
            self.select_mode(mode);
        } else {
            self.enter_mode(mode);
        }
        if mode == Mode::Read && !was_read {
            self.right_before_read = self.right.take();
            // An editing tool is put down so its open paragraph is applied, as `select_mode` does.
            if matches!(self.quick_tool, QuickTool::EditText | QuickTool::AddText) {
                self.put_down_edit_text();
            } else {
                self.quick_tool = QuickTool::Select;
            }
        } else if mode != Mode::Read && was_read {
            let before = self.right_before_read.take();
            if self.right.is_none() {
                self.right = before;
            }
        }
    }

    /// Opens a right panel, or closes it with `None`, because the user chose to. Closing Comments
    /// keeps comment tools from reopening it; opening it again lets them (#225). A choice made in
    /// Read mode replaces the pane saved on the way in, so leaving Read mode doesn't undo it.
    pub fn choose_right_panel(&mut self, panel: Option<RightPanel>) {
        if panel.is_some_and(|p| !self.profile.shows_panel(p)) {
            return;
        }
        if self.mode == Mode::Read {
            self.right_before_read = None;
        }
        if panel == Some(RightPanel::Comments) {
            self.comments_panel_closed = false;
        } else if panel.is_none() && self.right == Some(RightPanel::Comments) {
            self.comments_panel_closed = true;
        }
        if panel.is_some() {
            self.right_panel_closed = false;
        } else if self.right.is_some() {
            self.right_panel_closed = true;
        }
        self.right = panel;
        if let Some(panel) = panel {
            self.last_right = panel;
        }
    }

    /// F4: closes the side panel, or reopens the one open last.
    pub(crate) fn toggle_side_panel(&mut self) {
        if self.right.is_some() {
            self.choose_right_panel(None);
            return;
        }
        // The last panel, or the first one the profile shows when it hides that one.
        use RightPanel::*;
        let panel = std::iter::once(self.last_right)
            .chain([Comments, Bookmarks, Pages, Fields, Layers, Attachments, Signatures, Tags, Search])
            .find(|p| self.profile.shows_panel(*p));
        self.choose_right_panel(panel);
    }
}
