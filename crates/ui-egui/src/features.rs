//! Features: the parts of the app an embedding host can turn on or off. Every registered command,
//! every tool group and catalogue item, every document edit and every annotation belongs to exactly
//! one feature, so a [`crate::profile::UiProfile`] can offer "comments only" or "forms only" and the
//! interface hides the rest everywhere: menus, palette, toolbars, panels and shortcuts.
//!
//! The matches below are exhaustive on purpose: a new command, edit or shape upstream fails a test
//! or the build until it is given a feature.

use pdfcraft_engine::{Edit, Shape};

/// A feature of the app. [`Feature::key`] is its stable, host-facing name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Feature {
    /// Viewing: navigation, zoom, page display, read mode, full screen, layers. Always on.
    Viewer,
    /// Desktop-app features: opening, creating, combining and comparing files, tabs, recent
    /// files and folders, update checks, community links, guided actions, print production,
    /// search indexes and the AI assistant.
    App,
    /// Preferences, theme switching and the panel layout.
    Preferences,
    /// The command palette ("Find tools and commands").
    Palette,
    /// Text search.
    Search,
    /// Copying text and images out of the document.
    Copy,
    /// Document properties (read) and word count.
    DocumentInfo,
    /// Help: keyboard shortcuts and About.
    Help,
    /// Notes, text boxes, callouts, attachments, replies, review status, the comment list and
    /// comment import/export.
    Comments,
    /// Text markup: highlight, underline, strikethrough, squiggly, caret, replace text.
    Markup,
    /// Shapes: rectangle, oval, line, arrow, polygon, polyline, cloud.
    Shapes,
    /// Freehand ink and the eraser.
    Ink,
    /// Stamps, including custom stamps.
    Stamps,
    /// Measurement: distance, perimeter, area, scale.
    Measurement,
    /// Fill & Sign: typed text, marks, dates, drawn or typed signatures and initials.
    FillSign,
    /// Filling in form fields, clearing them, importing and exporting form data.
    FormFill,
    /// Preparing forms: adding and editing fields, tab order, field detection.
    FormDesign,
    /// Flattening comments and form fields into page content.
    Flatten,
    /// Page organisation: insert, delete, rotate, move, crop, number, replace pages.
    Pages,
    /// Editing bookmarks.
    Bookmarks,
    /// Editing page content: text, images, links, headers and footers, watermarks, backgrounds,
    /// Bates numbers, document metadata and initial view.
    EditContent,
    /// Redaction.
    Redaction,
    /// Passwords and permissions.
    Security,
    /// The signatures panel and validation (read-only).
    SignaturePanel,
    /// Signing and certifying with a digital ID.
    DigitalSignatures,
    /// Text recognition (OCR) and camera-scan clean-up.
    Ocr,
    /// Exporting to other formats, extracting and splitting pages.
    Export,
    /// Saving copies: Save As, optimized copies, saving attachments and reports.
    Download,
    /// PDF/A and other standards conversion.
    Standards,
    /// Printing.
    Print,
    /// Saving the document in place (Save, Revert).
    Save,
    /// Undo and redo.
    History,
    /// Accessibility checking, tagging and alternate text.
    Accessibility,
    /// Document JavaScript: running it, the console, editing scripts.
    JavaScript,
}

use Feature::*;

impl Feature {
    /// Every feature, in a stable order.
    pub const ALL: [Feature; 34] = [
        Viewer,
        App,
        Preferences,
        Palette,
        Search,
        Copy,
        DocumentInfo,
        Help,
        Comments,
        Markup,
        Shapes,
        Ink,
        Stamps,
        Measurement,
        FillSign,
        FormFill,
        FormDesign,
        Flatten,
        Pages,
        Bookmarks,
        EditContent,
        Redaction,
        Security,
        SignaturePanel,
        DigitalSignatures,
        Ocr,
        Export,
        Download,
        Standards,
        Print,
        Save,
        History,
        Accessibility,
        JavaScript,
    ];

    /// The host-facing name, such as `comments` or `fillSign`.
    pub const fn key(self) -> &'static str {
        match self {
            Viewer => "viewer",
            App => "app",
            Preferences => "preferences",
            Palette => "palette",
            Search => "search",
            Copy => "copy",
            DocumentInfo => "documentInfo",
            Help => "help",
            Comments => "comments",
            Markup => "markup",
            Shapes => "shapes",
            Ink => "ink",
            Stamps => "stamps",
            Measurement => "measurement",
            FillSign => "fillSign",
            FormFill => "formFill",
            FormDesign => "formDesign",
            Flatten => "flatten",
            Pages => "pages",
            Bookmarks => "bookmarks",
            EditContent => "editContent",
            Redaction => "redaction",
            Security => "security",
            SignaturePanel => "signaturePanel",
            DigitalSignatures => "digitalSignatures",
            Ocr => "ocr",
            Export => "export",
            Download => "download",
            Standards => "standards",
            Print => "print",
            Save => "save",
            History => "history",
            Accessibility => "accessibility",
            JavaScript => "javascript",
        }
    }

    /// The feature named `key` ([`Feature::key`]).
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }

    /// Whether the feature changes documents (as opposed to viewing, copying, printing or
    /// writing copies).
    pub const fn edits(self) -> bool {
        matches!(
            self,
            Comments
                | Markup
                | Shapes
                | Ink
                | Stamps
                | Measurement
                | FillSign
                | FormFill
                | FormDesign
                | Flatten
                | Pages
                | Bookmarks
                | EditContent
                | Redaction
                | Security
                | DigitalSignatures
                | Ocr
                | Standards
                | Accessibility
                | JavaScript
        )
    }

    const fn bit(self) -> u64 {
        1u64 << (self as u32)
    }
}

/// A set of features.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FeatureSet(u64);

impl FeatureSet {
    /// No feature (the viewer itself still works).
    pub const fn empty() -> Self {
        Self(Viewer.bit())
    }

    /// Every feature.
    pub fn all() -> Self {
        Feature::ALL.into_iter().collect()
    }

    pub const fn contains(self, feature: Feature) -> bool {
        // The viewer is always on.
        matches!(feature, Viewer) || self.0 & feature.bit() != 0
    }

    pub fn insert(&mut self, feature: Feature) {
        self.0 |= feature.bit();
    }

    pub fn remove(&mut self, feature: Feature) {
        if feature != Viewer {
            self.0 &= !feature.bit();
        }
    }

    /// The features in the set, in [`Feature::ALL`] order.
    pub fn iter(self) -> impl Iterator<Item = Feature> {
        Feature::ALL.into_iter().filter(move |f| self.contains(*f))
    }

    /// Whether any feature in the set changes documents.
    pub fn edits(self) -> bool {
        self.iter().any(Feature::edits)
    }
}

impl FromIterator<Feature> for FeatureSet {
    fn from_iter<I: IntoIterator<Item = Feature>>(iter: I) -> Self {
        let mut set = Self::empty();
        for f in iter {
            set.insert(f);
        }
        set
    }
}

/// The feature a command id belongs to: registered commands, and catalogue entries that are
/// not commands yet. `None` for an id that is neither.
pub fn command_feature(id: &str) -> Option<Feature> {
    let feature = match id {
        "a11y.alt_text"
        | "a11y.autotag"
        | "a11y.autotag_fields"
        | "a11y.check"
        | "a11y.find_untagged"
        | "a11y.reading_options"
        | "a11y.report"
        | "a11y.tags"
        | "a11y.reading_order"
        | "standards.pdfua" => Accessibility,
        "actions.distribution"
        | "actions.optimize_scans"
        | "actions.wizard"
        | "actions.archive"
        | "actions.make_accessible"
        | "create.blank"
        | "create.clipboard"
        | "create.file"
        | "create.images"
        | "create.multiple"
        | "create.multiple_separate"
        | "doc.compare"
        | "file.clear_recent"
        | "file.open"
        | "file.open_recent"
        | "file.pin_folder"
        | "form.merge_data"
        | "help.app_page"
        | "help.check_updates"
        | "help.discord"
        | "help.github"
        | "help.website"
        | "ocr.recognize_batch"
        | "page.combine"
        | "file.close"
        | "file.close_all"
        | "view.next_tab"
        | "view.previous_tab"
        | "ai.ask"
        | "ai.summary"
        | "ai.translate"
        | "preflight.run"
        | "prepress.convert_colors"
        | "prepress.hairlines"
        | "prepress.marks"
        | "prepress.output_preview"
        | "search.index" => App,
        "app.preferences" | "ui.dock.reset" | "view.theme" | "view.theme.dark" | "view.theme.light" | "view.theme.system" => Preferences,
        "bookmark.add" | "bookmark.from_structure" => Bookmarks,
        "comment.arrow" | "comment.circle" | "comment.cloud" | "comment.line" | "comment.polygon" | "comment.polyline" | "comment.square" => Shapes,
        "comment.attach" | "comment.callout" | "comment.export" | "comment.freetext" | "comment.hide_all" | "comment.import" | "comment.list"
        | "comment.note" | "comment.summarize" => Comments,
        "comment.caret" | "comment.highlight" | "comment.replace" | "comment.squiggly" | "comment.strikeout" | "comment.underline" => Markup,
        "comment.eraser" | "comment.ink" => Ink,
        "comment.flatten" | "form.flatten" => Flatten,
        "comment.stamp" => Stamps,
        "edit.advanced_search" | "edit.find" => Search,
        "edit.background"
        | "edit.background.remove"
        | "edit.background.update"
        | "edit.bates"
        | "edit.edit_text"
        | "edit.header_footer"
        | "edit.header_footer.remove"
        | "edit.header_footer.update"
        | "edit.image"
        | "edit.link"
        | "edit.links_from_urls"
        | "edit.remove_links"
        | "edit.text"
        | "edit.watermark"
        | "edit.watermark.remove"
        | "edit.watermark.update"
        | "edit.attach"
        | "page.transitions" => EditContent,
        "edit.column_select" | "edit.copy" | "edit.snapshot" => Copy,
        "edit.redo" | "edit.undo" => History,
        "export.all_images" | "export.docx" | "export.html" | "export.image" | "export.pptx" | "export.rtf" | "export.text" | "page.extract"
        | "page.split" | "export.ps" | "export.xlsx" => Export,
        "file.properties" | "file.word_count" | "protect.properties" => DocumentInfo,
        "file.revert" | "file.save" => Save,
        "file.save_as" | "optimize.advanced" | "optimize.reduce" | "optimize.audit" => Download,
        "form.add.button"
        | "form.add.checkbox"
        | "form.add.combo"
        | "form.add.date"
        | "form.add.image"
        | "form.add.list"
        | "form.add.radio"
        | "form.add.signature"
        | "form.add.text"
        | "form.add.text_table"
        | "form.calculation_order"
        | "form.detect"
        | "form.field.properties"
        | "form.prepare"
        | "form.tab_order.column"
        | "form.tab_order.row"
        | "form.tab_order.structure" => FormDesign,
        "form.clear" | "form.export_data" | "form.fields" | "form.import_data" => FormFill,
        "help.about" | "help.shortcuts" => Help,
        "measure.area" | "measure.distance" | "measure.export" | "measure.info" | "measure.perimeter" | "measure.scale" | "measure.snap"
        | "measure.geo" => Measurement,
        "ocr.enhance_camera" | "ocr.recognize" | "ocr.correct" | "ocr.enhance" => Ocr,
        "page.boxes" | "page.copy" | "page.crop" | "page.cut" | "page.delete" | "page.duplicate" | "page.insert" | "page.insert_blank"
        | "page.number" | "page.organize" | "page.paste" | "page.replace" | "page.reverse" | "page.rotate" | "page.rotate_ccw"
        | "page.rotate_dialog" => Pages,
        "print.dialog" => Print,
        "protect.password" | "protect.remove" | "protect.unlock" => Security,
        "protect.remove_hidden"
        | "redact.apply"
        | "redact.clear"
        | "redact.mark"
        | "redact.pages"
        | "redact.properties"
        | "redact.sanitize"
        | "redact.search" => Redaction,
        "sign.certify" | "sign.certify_invisible" | "sign.digital" | "protect.certificate" | "sign.timestamp" => DigitalSignatures,
        "sign.fill.check"
        | "sign.fill.cross"
        | "sign.fill.date"
        | "sign.fill.dot"
        | "sign.fill.initials"
        | "sign.fill.initials.change"
        | "sign.fill.initials.remove"
        | "sign.fill.line"
        | "sign.fill.signature"
        | "sign.fill.signature.change"
        | "sign.fill.signature.remove"
        | "sign.fill.text" => FillSign,
        "sign.panel" | "sign.validate" => SignaturePanel,
        "standards.pdfa" | "standards.pdfx" => Standards,
        "tools.document_js" | "tools.js_console" | "js.actions" | "js.console" | "js.document" => JavaScript,
        "view.continuous"
        | "view.fit_one_page"
        | "view.fit_visible"
        | "view.fit_width_scrolling"
        | "view.focus_page_input"
        | "view.full_screen"
        | "view.layout.cover"
        | "view.layout.single"
        | "view.layout.two_up"
        | "view.marquee_zoom"
        | "view.next_view"
        | "view.previous_view"
        | "view.read_mode" => Viewer,
        "view.palette" => Palette,
        _ => return None,
    };
    Some(feature)
}

/// The feature a tool group belongs to: the one its catalogue entries serve. A group shows when
/// its feature is on and at least one of its entries is allowed.
pub fn group_feature(id: &str) -> Option<Feature> {
    let feature = match id {
        "comment" => Comments,
        "stamp" => Stamps,
        "measure" => Measurement,
        "fill_sign" => FillSign,
        "form" => FormDesign,
        "organize" => Pages,
        "edit" => EditContent,
        "redact" => Redaction,
        "protect" => Security,
        "certificate" => DigitalSignatures,
        "scan" => Ocr,
        "export" => Export,
        "compress" => Download,
        "standards" => Standards,
        "accessibility" => Accessibility,
        "javascript" => JavaScript,
        "create" | "combine" | "compare" | "actions" | "print_production" | "search_index" | "ai" => App,
        _ => return None,
    };
    Some(feature)
}

/// The feature that makes a new annotation of this shape.
pub fn shape_feature(shape: &Shape) -> Feature {
    match shape {
        Shape::Note { .. } | Shape::TextBox { .. } | Shape::Callout { .. } | Shape::Attachment { .. } => Comments,
        Shape::TextMarkup { .. } | Shape::Caret { .. } => Markup,
        Shape::Rectangle { .. } | Shape::Oval { .. } | Shape::Line { .. } | Shape::Polygon { .. } | Shape::PolyLine { .. } => Shapes,
        Shape::Ink { .. } => Ink,
        Shape::Stamp { .. } | Shape::CustomStamp { .. } => Stamps,
        Shape::Typewriter { .. } | Shape::Mark { .. } | Shape::Signature { .. } | Shape::TypedSignature { .. } => FillSign,
        Shape::Redact { .. } => Redaction,
    }
}

/// The feature that owns an existing annotation, from its subtype (`Highlight`), intent and
/// whether it is a Fill & Sign mark.
pub fn annotation_feature(subtype: &str, intent: Option<&str>, fill_sign: bool) -> Feature {
    if fill_sign {
        return FillSign;
    }
    if intent.is_some_and(|i| i.ends_with("Dimension")) {
        return Measurement;
    }
    match subtype {
        "Highlight" | "Underline" | "StrikeOut" | "Squiggly" | "Caret" => Markup,
        "Square" | "Circle" | "Line" | "Polygon" | "PolyLine" => Shapes,
        "Ink" => Ink,
        "Stamp" => Stamps,
        "Redact" => Redaction,
        "Link" => EditContent,
        "Widget" => FormDesign,
        // Text, FreeText, FileAttachment, Sound, Popup and anything else: comments.
        _ => Comments,
    }
}

/// Whether every change `edit` makes is allowed: `allowed` says which features are on, and
/// `annotation` gives the feature of the existing annotation at (page, index), if any.
pub fn edit_allowed(edit: &Edit, allowed: &dyn Fn(Feature) -> bool, annotation: &dyn Fn(usize, usize) -> Option<Feature>) -> bool {
    let existing = |page: usize, index: usize| allowed(annotation(page, index).unwrap_or(Comments));
    match edit {
        Edit::Batch { edits, .. } => edits.iter().all(|e| edit_allowed(e, allowed, annotation)),
        Edit::AddAnnotation(new) => allowed(shape_feature(&new.shape)),
        Edit::DeleteAnnotation { page, index }
        | Edit::SetAnnotationContents { page, index, .. }
        | Edit::EraseInk { page, index, .. }
        | Edit::MarkAnnotation { page, index, .. }
        | Edit::LockAnnotation { page, index, .. }
        | Edit::MoveAnnotation { page, index, .. }
        | Edit::ResizeAnnotation { page, index, .. }
        | Edit::StyleAnnotation { page, index, .. }
        | Edit::FillAnnotation { page, index, .. }
        | Edit::FontSizeAnnotation { page, index, .. }
        | Edit::SetAnnotationInfo { page, index, .. } => existing(*page, *index),
        // Replies and review status are part of a comment thread, on any annotation.
        Edit::ReplyToAnnotation { .. } | Edit::SetAnnotationStatus { .. } => allowed(Comments),
        other => allowed(simple_edit_feature(other)),
    }
}

/// The features `edit` uses (every part of a batch), with `annotation` as in [`edit_allowed`].
pub fn edit_features(edit: &Edit, annotation: &dyn Fn(usize, usize) -> Option<Feature>) -> FeatureSet {
    let used = std::cell::Cell::new(FeatureSet::empty());
    edit_allowed(
        edit,
        &|f| {
            let mut set = used.get();
            set.insert(f);
            used.set(set);
            true
        },
        annotation,
    );
    used.get()
}

/// The feature of an edit that doesn't depend on an existing annotation.
fn simple_edit_feature(edit: &Edit) -> Feature {
    match edit {
        Edit::RotatePages { .. }
        | Edit::DeletePages { .. }
        | Edit::MovePages { .. }
        | Edit::ReversePages { .. }
        | Edit::InsertBlankPage { .. }
        | Edit::DuplicatePages { .. }
        | Edit::ReplacePages { .. }
        | Edit::SetPageBox { .. }
        | Edit::SetTransition { .. }
        | Edit::InsertPagesFrom { .. }
        | Edit::NumberPages { .. } => Pages,
        Edit::AddBookmark { .. }
        | Edit::RenameBookmark { .. }
        | Edit::DeleteBookmark { .. }
        | Edit::MoveBookmark { .. }
        | Edit::SetBookmarkPage { .. }
        | Edit::BookmarksFromStructure => Bookmarks,
        Edit::AddMeasurement(_) | Edit::SetMeasurementScale { .. } => Measurement,
        Edit::AddCustomStamp { .. } => Stamps,
        Edit::ReplaceText { .. } => Markup,
        // Comment data (XFDF, FDF); form data is gated by its command (form.import_data).
        Edit::ImportData { .. } => Comments,
        Edit::SetFieldValue { .. } | Edit::ResetForm { .. } | Edit::SetFieldImage { .. } | Edit::XfaEvent { .. } => FormFill,
        Edit::AddField { .. }
        | Edit::SetFieldProps { .. }
        | Edit::DuplicateField { .. }
        | Edit::DeleteField { .. }
        | Edit::SetTabOrder { .. }
        | Edit::MoveInTabOrder { .. }
        | Edit::SetCalculationOrder { .. }
        | Edit::XfaToStatic => FormDesign,
        Edit::AutoTagText
        | Edit::SetAltText { .. }
        | Edit::AutoTagFormFields
        | Edit::SetTagProperty { .. }
        | Edit::MoveTag { .. }
        | Edit::MarkDecorative { .. }
        | Edit::MarkTagArtifact { .. } => Accessibility,
        Edit::SetDocumentScript { .. } | Edit::SetFieldScript { .. } | Edit::ApplyScriptChanges { .. } => JavaScript,
        Edit::ConvertPdfA { .. } => Standards,
        Edit::AddOcrText { .. } => Ocr,
        Edit::SetInfo { .. }
        | Edit::SetInitialView { .. }
        | Edit::EditTextLine { .. }
        | Edit::EditPageImage { .. }
        | Edit::MoveObjects { .. }
        | Edit::EditTextBlock { .. }
        | Edit::RegisterTextFont { .. }
        | Edit::AddHeaderFooter { .. }
        | Edit::AddWatermark { .. }
        | Edit::AddBackground { .. }
        | Edit::RemoveMarks { .. }
        | Edit::AddText { .. }
        | Edit::AddImage { .. }
        | Edit::UpdateContent { .. }
        | Edit::DeleteContent { .. }
        | Edit::ReplaceImage { .. }
        | Edit::AddLink { .. }
        | Edit::SetLink { .. }
        | Edit::DeleteLink { .. }
        | Edit::RemoveLinks { .. }
        | Edit::AddLinks { .. } => EditContent,
        Edit::ApplyRedactions { .. } | Edit::ClearRedactions | Edit::RemoveHidden { .. } | Edit::Sanitize => Redaction,
        Edit::Flatten { .. } | Edit::FlattenFillSign => Flatten,
        Edit::Protect(_) | Edit::RemoveProtection => Security,
        // Handled by edit_allowed; listed so this match stays exhaustive.
        Edit::Batch { .. }
        | Edit::AddAnnotation(_)
        | Edit::DeleteAnnotation { .. }
        | Edit::SetAnnotationContents { .. }
        | Edit::ReplyToAnnotation { .. }
        | Edit::SetAnnotationStatus { .. }
        | Edit::EraseInk { .. }
        | Edit::MarkAnnotation { .. }
        | Edit::LockAnnotation { .. }
        | Edit::MoveAnnotation { .. }
        | Edit::ResizeAnnotation { .. }
        | Edit::StyleAnnotation { .. }
        | Edit::FillAnnotation { .. }
        | Edit::FontSizeAnnotation { .. }
        | Edit::SetAnnotationInfo { .. } => Comments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_and_catalogue_entry_has_a_feature() {
        for c in pdfcraft_engine::commands::COMMANDS {
            assert!(command_feature(c.id).is_some(), "command {} has no feature", c.id);
        }
        for g in pdfcraft_engine::catalog::TOOL_GROUPS {
            assert!(group_feature(g.id).is_some(), "tool group {} has no feature", g.id);
            for s in g.sections {
                for i in s.items {
                    assert!(command_feature(i.command).is_some(), "catalogue entry {} ({}) has no feature", i.command, g.id);
                    for a in i.actions {
                        assert!(command_feature(a).is_some(), "catalogue action {a} has no feature");
                    }
                }
            }
        }
    }

    #[test]
    fn keys_round_trip_and_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for f in Feature::ALL {
            assert!(seen.insert(f.key()), "duplicate key {}", f.key());
            assert_eq!(Feature::from_key(f.key()), Some(f));
        }
        assert_eq!(Feature::from_key("nope"), None);
        assert!(Feature::ALL.len() < 64, "FeatureSet is a u64");
    }

    #[test]
    fn sets_keep_the_viewer_and_report_editing() {
        let mut set = FeatureSet::empty();
        assert!(set.contains(Viewer) && !set.contains(Comments) && !set.edits());
        set.remove(Viewer);
        assert!(set.contains(Viewer));
        set.insert(Print);
        assert!(!set.edits());
        set.insert(Comments);
        assert!(set.edits());
        assert_eq!(FeatureSet::all().iter().count(), Feature::ALL.len());
    }

    #[test]
    fn existing_annotations_belong_to_their_kind() {
        assert_eq!(annotation_feature("Highlight", None, false), Markup);
        assert_eq!(annotation_feature("Line", Some("LineDimension"), false), Measurement);
        assert_eq!(annotation_feature("FreeText", None, true), FillSign);
        assert_eq!(annotation_feature("Square", None, false), Shapes);
        assert_eq!(annotation_feature("Text", None, false), Comments);
    }

    #[test]
    fn edits_follow_their_target() {
        let only = |f: Feature| move |g: Feature| g == f;
        let shape_at = |_: usize, _: usize| Some(Shapes);
        let delete = Edit::DeleteAnnotation { page: 0, index: 0 };
        assert!(edit_allowed(&delete, &only(Shapes), &shape_at));
        assert!(!edit_allowed(&delete, &only(Comments), &shape_at));
        let batch = Edit::Batch { label: String::new(), edits: vec![delete.clone(), Edit::DeletePages { pages: vec![0] }] };
        assert!(!edit_allowed(&batch, &only(Shapes), &shape_at), "a batch needs every part allowed");
    }
}
