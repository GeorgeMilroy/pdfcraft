//! Left tool panel (All tools + tool sub-panels, generated from the engine catalogue) and the
//! right-hand panels (Comments, Bookmarks, Pages, Fields, Layers, Attachments).

use egui::{Align, Align2, Color32, CornerRadius, Layout, Rect, Sense, Stroke, pos2, vec2};
use pdfcraft_engine::catalog::{self, Availability, TOOL_GROUPS, ToolGroup, ToolItem};
use pdfcraft_render::{DocInfo, FieldKind, OutlineItem};

use crate::profile::UiProfile;
use crate::theme::{self, Tokens};
use crate::{LeftPanel, PdfCraftApp, RightPanel, icons, widgets};

/// Whether the profile offers command `id`: allowed, and its feature on (comments-only turns off
/// every feature but the comment ones, whatever a command's permission says).
pub(crate) fn offers(profile: &UiProfile, id: &str) -> bool {
    profile.allows_id(id) && crate::features::command_feature(id).is_none_or(|f| profile.allows_feature(f))
}

/// Whether a catalogue entry shows: [`UiProfile::shows_item`], and its command (or one of its
/// actions) [`offers`]ed.
fn item_shown(profile: &UiProfile, item: &catalog::ToolItem) -> bool {
    profile.shows_item(item) && (offers(profile, item.command) || item.actions.iter().any(|a| offers(profile, a)))
}

/// Whether a tool group shows: not hidden, its feature on, and at least one entry shown.
pub(crate) fn group_shown(profile: &UiProfile, g: &ToolGroup) -> bool {
    profile.shows_group(g)
        && crate::features::group_feature(g.id).is_none_or(|f| profile.allows_feature(f))
        && g.sections.iter().flat_map(|s| s.items.iter()).any(|item| item_shown(profile, item))
}

pub(super) mod all_tools_ui;
mod bookmarks;
pub(super) mod comments_panel;
mod edit;
pub(super) mod format_text_ui;
pub(super) mod organize_panel;
pub(super) mod page_selection_ui;
mod pages;
mod right;
pub(super) mod search_ui;
pub mod side_panels;
mod tools;

pub use bookmarks::BmAction;
pub(crate) use bookmarks::ellipsized_prefix;
pub use right::right_panel;
pub use tools::left_panel;
pub(crate) use tools::panel_header;

// ───────────────────────────────────────────────────────────────────────────── right panels

pub(crate) enum Nav {
    Page(usize),
    /// A bookmark's destination: its page and where on it.
    Dest(usize, pdfcraft_render::DestView),
    Flash(usize, [f32; 4]),
}

pub fn human_size(n: usize) -> String {
    match n {
        n if n >= 1 << 20 => format!("{:.1} MB", n as f64 / (1u64 << 20) as f64),
        n if n >= 1 << 10 => format!("{:.1} KB", n as f64 / (1u64 << 10) as f64),
        n => byte_count(n),
    }
}

/// An exact size in bytes, with the count's own plural form.
pub fn byte_count(n: usize) -> String {
    crate::i18n::trn(crate::i18n::current(), n as u64, "{n} byte", "{n} bytes")
}
