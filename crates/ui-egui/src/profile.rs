//! What an embedding host lets the interface do: a locked view-only mode, comments-only editing,
//! and hidden commands, tool groups and interface parts. [`UiProfile::default`] hides and locks
//! nothing, which is the app as it ships.
//!
//! The profile is enforced where things happen, not only where they are drawn: a hidden command
//! doesn't run from a menu, a shortcut or the command palette, and a refused edit is refused by
//! [`crate::PdfCraftApp::apply_edit`] whatever asked for it.

use std::collections::BTreeSet;

use pdfcraft_engine::Edit;
use pdfcraft_engine::commands::{CommandSpec, Needs};

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
}

impl UiProfile {
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
        if self.read_only && changes_documents(spec.needs) {
            return false;
        }
        if matches!(spec.id, "file.save" | "file.save_as") && !self.allows_save() {
            return false;
        }
        !(self.comments_only && changes_more_than_comments(spec.needs))
    }

    /// [`Self::allows_command`] by command id; an id that is not a registered command is allowed
    /// unless hidden.
    pub fn allows_id(&self, id: &str) -> bool {
        match pdfcraft_engine::commands::command(id) {
            Some(spec) => self.allows_command(spec),
            None => !self.hides(id),
        }
    }

    /// Whether the edit may be applied.
    pub fn allows_edit(&self, edit: &Edit) -> bool {
        if self.read_only {
            return false;
        }
        !self.comments_only || is_comment_edit(edit)
    }

    /// Whether documents may be saved.
    pub fn allows_save(&self) -> bool {
        !self.read_only && !(self.hides("file.save") && self.hides("file.save_as"))
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
