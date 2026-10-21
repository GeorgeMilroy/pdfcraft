//! What an embedding host lets the interface do: a locked view-only mode, comments-only editing,
//! and hidden commands, tool groups and interface parts. [`UiProfile::default`] hides and locks
//! nothing, which is the app as it ships.
//!
//! The profile is enforced where things happen, not only where they are drawn: a hidden command
//! doesn't run from a menu, a shortcut or the command palette, and a refused edit is refused by
//! [`crate::PdfCraftApp::apply_edit`] whatever asked for it.

use std::collections::BTreeSet;

use pdfcraft_engine::Edit;
use pdfcraft_engine::catalog::{Availability, ToolGroup, ToolItem};
use pdfcraft_engine::commands::{CommandSpec, Needs};

use crate::features::{self, Feature, FeatureSet};

/// Interface parts that can be hidden, besides commands and tool groups.
pub mod parts {
    /// The tab strip (document tabs, Open, the theme switch).
    pub const TABS: &str = "ui:tabs";
    /// The workspace mode tabs (All tools, Read, Edit, Convert, E-Sign).
    pub const MODE_TABS: &str = "ui:mode_tabs";
    /// The main menu button.
    pub const MENU: &str = "ui:menu";
    /// The "Find tools and commands" search box and its palette.
    pub const SEARCH: &str = "ui:search";
    /// The Home screen shown when no document is open.
    pub const HOME: &str = "ui:home";
    /// The right-hand rail (panel buttons, page navigation, zoom).
    pub const RAIL: &str = "ui:rail";
    /// The rail's zoom and page display controls.
    pub const RAIL_ZOOM: &str = "ui:rail_zoom";
    /// The rail's page navigation (previous/next page, page number).
    pub const RAIL_PAGE_NAV: &str = "ui:rail_page_nav";
    /// The floating tool bar beside the page (Select, Hand, comment and Fill & Sign tools).
    pub const QUICK_BAR: &str = "ui:quick_bar";
    /// The Tools panel on the left.
    pub const TOOLS_PANEL: &str = "ui:tools_panel";
    /// Managing the custom stamp library (Create custom stamp…, Delete stamp); its stamps can
    /// still be placed.
    pub const STAMPS_CUSTOM: &str = "stamps.custom";
    /// A right-hand panel, as `panel:<name>`: `comments`, `bookmarks`, `pages`, `fields`,
    /// `layers`, `attachments`, `signatures`, `accessibility`, `tags`, `search`, `compare`
    /// ([`super::UiProfile::shows_panel`]).
    pub const PANEL_PREFIX: &str = "panel:";
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiProfile {
    /// Nothing in a document can change from the interface: no edit, undo, redo or save, and no
    /// editing tool; the workspace stays in Read mode.
    pub read_only: bool,
    /// Only comments change: notes, markup, shapes, ink, stamps, Fill & Sign marks, measurements
    /// and imported comment data. Pages, content, forms and security don't.
    pub comments_only: bool,
    /// Hidden command ids (`file.save`), tool groups (`group:export`) and interface parts
    /// ([`parts`]). A hidden command does not run.
    pub hidden: BTreeSet<String>,
    /// While the Tools panel is closed, or hidden by Read mode, a rail on its side brings it
    /// back: one button reopens it as it was, then one per tool group shown opens that group.
    /// For hosts that hide the mode tabs, which otherwise bring it back. Not with `read_only`.
    pub tools_rail: bool,
    /// The features the interface offers ([`crate::features`]); `None` is every feature, the app
    /// as it ships. A feature that is off is hidden everywhere: its commands, tool groups and
    /// catalogue entries, panels and buttons, and its edits are refused.
    pub features: Option<FeatureSet>,
    /// Leave out catalogue entries that are not available yet (planned for a later milestone or
    /// needing a provider), instead of listing them with their milestone.
    pub hide_planned: bool,
    /// Panels can't be moved, floated, closed from their tab or rearranged.
    pub lock_layout: bool,
    /// Whether links to web pages (URI actions) may open, after the usual prompt; `None` keeps
    /// the app's behaviour (they may).
    pub external_links: Option<bool>,
}

impl UiProfile {
    /// Whether `feature` is on: in [`Self::features`], and not an editing feature in a
    /// read-only profile (nor a non-comment one in a comments-only profile).
    pub fn allows_feature(&self, feature: Feature) -> bool {
        use Feature::*;
        if self.read_only && feature.edits() {
            return false;
        }
        if self.comments_only && feature.edits() && !matches!(feature, Comments | Markup | Shapes | Ink | Stamps | Measurement | FillSign) {
            return false;
        }
        self.features.is_none_or(|set| set.contains(feature))
    }

    /// Whether interface part `id` ([`parts`]) shows.
    pub fn shows_part(&self, id: &str) -> bool {
        !self.hides(id)
    }

    /// Whether a right-hand panel can show: not hidden as `panel:<name>`, and its feature is on.
    /// Panels that only list (comments, bookmarks, pages, fields, layers, attachments) belong to
    /// the viewer; what they let you change is gated where it happens.
    pub fn shows_panel(&self, panel: crate::RightPanel) -> bool {
        use crate::RightPanel;
        let (name, feature) = match panel {
            RightPanel::Comments => ("comments", Feature::Viewer),
            RightPanel::Bookmarks => ("bookmarks", Feature::Viewer),
            RightPanel::Pages => ("pages", Feature::Viewer),
            RightPanel::Fields => ("fields", Feature::Viewer),
            RightPanel::Layers => ("layers", Feature::Viewer),
            RightPanel::Attachments => ("attachments", Feature::Viewer),
            RightPanel::Signatures => ("signatures", Feature::SignaturePanel),
            RightPanel::Accessibility => ("accessibility", Feature::Accessibility),
            RightPanel::Tags => ("tags", Feature::Accessibility),
            RightPanel::Search => ("search", Feature::Search),
            RightPanel::Compare => ("compare", Feature::App),
        };
        self.allows_feature(feature) && !self.hides(&format!("{}{name}", parts::PANEL_PREFIX))
    }

    /// Whether the tool group shows: not hidden, and at least one of its entries is allowed.
    pub fn shows_group(&self, group: &ToolGroup) -> bool {
        !self.hides_group(group.id) && group.sections.iter().flat_map(|s| s.items.iter()).any(|item| self.shows_item(item))
    }

    /// Whether a catalogue entry shows: its command (or, for a menu entry, one of its actions)
    /// is allowed, and it is available unless [`Self::hide_planned`] says to leave it out.
    pub fn shows_item(&self, item: &ToolItem) -> bool {
        if self.hide_planned && item.availability != Availability::Ready {
            return false;
        }
        self.allows_id(item.command) || item.actions.iter().any(|a| self.allows_id(a))
    }

    /// Whether `id` (a command, `group:<tool group>` or a [`parts`] id) is hidden.
    pub fn hides(&self, id: &str) -> bool {
        self.hidden.contains(id)
    }

    /// Whether the tool group `group` (catalog id, such as `export`) is hidden.
    pub fn hides_group(&self, group: &str) -> bool {
        self.hidden.iter().any(|h| h.strip_prefix("group:") == Some(group))
    }

    /// Whether the command may show and run.
    pub fn allows_command(&self, spec: &CommandSpec) -> bool {
        if self.hides(spec.id) {
            return false;
        }
        // An invisible certifying signature is a way of certifying.
        if spec.id == "sign.certify_invisible" && self.hides("sign.certify") {
            return false;
        }
        // Reading options are a tab of Document Properties.
        if spec.id == "a11y.reading_options" && !self.allows_id("file.properties") {
            return false;
        }
        if self.features.is_some() && !features::command_feature(spec.id).is_some_and(|f| self.allows_feature(f)) {
            return false;
        }
        if self.read_only && changes_documents(spec.needs) {
            return false;
        }
        // Commands that need nothing of the document can still belong to an editing feature (the
        // saved signature's Change and Remove): the legacy flags refuse those as allows_feature does.
        if (self.read_only || self.comments_only) && features::command_feature(spec.id).is_some_and(|f| !self.allows_feature(f)) {
            return false;
        }
        if matches!(spec.id, "file.save" | "file.save_as") && !self.allows_save() {
            return false;
        }
        !(self.comments_only && changes_more_than_comments(spec.needs))
    }

    /// [`Self::allows_command`] by command id. An id that is not a registered command (a
    /// catalogue entry still to come) is allowed unless hidden or its feature is off.
    pub fn allows_id(&self, id: &str) -> bool {
        match pdfcraft_engine::commands::command(id) {
            Some(spec) => self.allows_command(spec),
            None => !self.hides(id) && (self.features.is_none() || features::command_feature(id).is_some_and(|f| self.allows_feature(f))),
        }
    }

    /// Whether the edit may be applied. Edits of an existing annotation count as the feature of
    /// that annotation's kind, unknown here (use [`Self::allows_edit_in`] with the document).
    pub fn allows_edit(&self, edit: &Edit) -> bool {
        self.allows_edit_in(edit, &|_, _| None)
    }

    /// [`Self::allows_edit`], where `annotation` gives the feature of the existing annotation at
    /// (page, index), so moving a shape needs shapes and deleting a highlight needs markup.
    pub fn allows_edit_in(&self, edit: &Edit, annotation: &dyn Fn(usize, usize) -> Option<Feature>) -> bool {
        if self.read_only {
            return false;
        }
        if self.comments_only && !is_comment_edit(edit) {
            return false;
        }
        self.features.is_none() || features::edit_allowed(edit, &|f| self.allows_feature(f), annotation)
    }

    /// Whether documents may be saved.
    pub fn allows_save(&self) -> bool {
        !self.read_only && self.allows_feature(Feature::Save) && !(self.hides("file.save") && self.hides("file.save_as"))
    }
}

/// Commands that need a permission to change the document.
fn changes_documents(needs: Needs) -> bool {
    use Needs::*;
    matches!(
        needs,
        Assembly
            | Modification
            | Annotate
            | FillForms
            | Marks(_)
            | HasComments
            | HasFields
            | HasRedactions
            | Security
            | ProtectedSecurity
            | Undo
            | Redo
    )
}

/// Commands whose changes go beyond comments.
fn changes_more_than_comments(needs: Needs) -> bool {
    use Needs::*;
    matches!(needs, Assembly | Modification | FillForms | Marks(_) | HasComments | HasFields | HasRedactions | Security | ProtectedSecurity)
}

/// Edits that only add, change or remove comments (the engine's "annotate" permission), and
/// imported comment data.
pub fn is_comment_edit(edit: &Edit) -> bool {
    match edit {
        Edit::AddMeasurement(_)
        | Edit::AddAnnotation(_)
        | Edit::AddCustomStamp { .. }
        | Edit::DeleteAnnotation { .. }
        | Edit::SetAnnotationContents { .. }
        | Edit::ReplyToAnnotation { .. }
        | Edit::SetAnnotationStatus { .. }
        | Edit::MarkAnnotation { .. }
        | Edit::ReplaceText { .. }
        | Edit::EraseInk { .. }
        | Edit::LockAnnotation { .. }
        | Edit::MoveAnnotation { .. }
        | Edit::ResizeAnnotation { .. }
        | Edit::StyleAnnotation { .. }
        | Edit::FillAnnotation { .. }
        | Edit::FontSizeAnnotation { .. }
        | Edit::SetAnnotationInfo { .. }
        | Edit::SetMeasurementScale { .. }
        | Edit::ImportData { .. } => true,
        Edit::Batch { edits, .. } => edits.iter().all(is_comment_edit),
        _ => false,
    }
}
