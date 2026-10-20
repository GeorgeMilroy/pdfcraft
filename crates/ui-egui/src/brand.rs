//! The product identity the interface shows: name, logos, accent colour, About text and the
//! settings key. [`Brand::default`] is PdfCraft as it ships; an app embedding this crate under
//! another name (a white-label build) calls [`set`] once, before the first frame.
//!
//! ArtCraft's marks (docs/brand/) are Storyteller's trademarks: a modified build that is
//! distributed must replace them, which a custom [`Brand`] does without editing this crate.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

/// The name every PdfCraft string uses; a custom brand's name takes its place on screen.
pub const DEFAULT_APP_NAME: &str = "PdfCraft";

/// Distinct rebranded strings kept for the life of the app (each is leaked once, see
/// [`rebrand`]). The interface has a few dozen; the cap only guards against a runaway caller.
const MAX_INTERNED: usize = 2048;

/// An image the interface draws: an egui image URI (unique per image, such as
/// `bytes://brand-logo.svg`), its bytes (SVG or PNG) and its alternative text.
#[derive(Clone)]
pub struct BrandImage {
    pub uri: String,
    pub bytes: egui::load::Bytes,
    pub alt: String,
}

impl BrandImage {
    pub fn new_static(uri: impl Into<String>, bytes: &'static [u8], alt: impl Into<String>) -> Self {
        Self { uri: uri.into(), bytes: egui::load::Bytes::Static(bytes), alt: alt.into() }
    }
}

impl std::fmt::Debug for BrandImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrandImage").field("uri", &self.uri).field("len", &self.bytes.len()).field("alt", &self.alt).finish()
    }
}

/// How the app names and presents itself.
#[derive(Clone, Debug)]
pub struct Brand {
    /// Shown in the window title, the About dialog and every message that names the app.
    pub app_name: String,
    /// Version shown in About; `None` shows PdfCraft's version.
    pub version: Option<String>,
    /// Wordmark on light and dark backgrounds (About's "Part of" row); `None` hides the row.
    pub logo_light: Option<BrandImage>,
    pub logo_dark: Option<BrandImage>,
    /// Square mark (About, Home); `None` draws nothing.
    pub mark: Option<BrandImage>,
    /// Accent colour for the light and the dark theme; `None` keeps the theme's blue.
    pub accent: Option<(egui::Color32, egui::Color32)>,
    /// About's description; `None` keeps PdfCraft's.
    pub about_text: Option<String>,
    /// A link under the About text; `None` shows none.
    pub about_url: Option<String>,
    /// Whether the community links (Discord, web page, GitHub, website) appear: in Help, About,
    /// Home and the tab strip.
    pub community_links: bool,
    /// The key the settings are stored under (eframe storage: localStorage on the web).
    pub storage_key: String,
}

impl Default for Brand {
    fn default() -> Self {
        let (logo_light, logo_dark, mark) = default_marks();
        Self {
            app_name: DEFAULT_APP_NAME.to_owned(),
            version: None,
            logo_light,
            logo_dark,
            mark,
            accent: None,
            about_text: None,
            about_url: None,
            community_links: true,
            storage_key: "pdfcraft".to_owned(),
        }
    }
}

/// The ArtCraft wordmark (light, dark) and mark, when the `artcraft-marks` feature is on.
#[cfg(feature = "artcraft-marks")]
fn default_marks() -> (Option<BrandImage>, Option<BrandImage>, Option<BrandImage>) {
    (
        Some(BrandImage::new_static("bytes://artcraft-logo.svg", include_bytes!("../../../docs/brand/artcraft-logo.svg"), "ArtCraft")),
        Some(BrandImage::new_static("bytes://artcraft-logo-white.svg", include_bytes!("../../../docs/brand/artcraft-logo-white.svg"), "ArtCraft")),
        Some(BrandImage::new_static("bytes://artcraft-mark.svg", include_bytes!("../../../docs/brand/artcraft-mark.svg"), "ArtCraft")),
    )
}

/// Without the `artcraft-marks` feature the default brand has no images.
#[cfg(not(feature = "artcraft-marks"))]
fn default_marks() -> (Option<BrandImage>, Option<BrandImage>, Option<BrandImage>) {
    (None, None, None)
}

impl Brand {
    /// Whether strings naming PdfCraft are rewritten with [`Brand::app_name`].
    fn renames(&self) -> bool {
        self.app_name != DEFAULT_APP_NAME
    }

    /// The window title for a document (or for none).
    pub fn window_title(&self, document: Option<&str>) -> String {
        match document {
            Some(name) => format!("{name} — {}", self.app_name),
            None => self.app_name.clone(),
        }
    }

    /// `s` with "PdfCraft" replaced by the app name (a new string), or `None` when unchanged.
    pub fn renamed(&self, s: &str) -> Option<String> {
        (self.renames() && s.contains(DEFAULT_APP_NAME)).then(|| s.replace(DEFAULT_APP_NAME, &self.app_name))
    }
}

static BRAND: RwLock<Option<Arc<Brand>>> = RwLock::new(None);
static INTERNED: Mutex<Option<HashMap<String, &'static str>>> = Mutex::new(None);

/// Use `brand` from now on (call before the first frame; later calls apply on the next frame).
pub fn set(brand: Brand) {
    let mut slot = BRAND.write().unwrap_or_else(std::sync::PoisonError::into_inner);
    *slot = Some(Arc::new(brand));
    // Strings rebranded under the previous name no longer apply.
    if let Ok(mut interned) = INTERNED.lock() {
        *interned = None;
    }
}

/// The brand in use: the one given to [`set`], or [`Brand::default`].
pub fn current() -> Arc<Brand> {
    let slot = BRAND.read().unwrap_or_else(std::sync::PoisonError::into_inner);
    match slot.as_ref() {
        Some(b) => Arc::clone(b),
        None => Arc::new(Brand::default()),
    }
}

/// `s` as the current brand shows it. With the default brand, or when `s` doesn't name PdfCraft,
/// this is `s` itself. Otherwise it is the renamed string, kept for the rest of the app's life
/// so it can be borrowed like a catalog entry (bounded by [`MAX_INTERNED`]; past it, `s`).
pub fn rebrand(s: &str) -> &str {
    if !s.contains(DEFAULT_APP_NAME) {
        return s;
    }
    let brand = current();
    let Some(renamed) = brand.renamed(s) else { return s };
    let Ok(mut guard) = INTERNED.lock() else { return s };
    let map = guard.get_or_insert_with(HashMap::new);
    if let Some(&hit) = map.get(s) {
        return hit;
    }
    if map.len() >= MAX_INTERNED {
        return s;
    }
    let leaked: &'static str = Box::leak(renamed.into_boxed_str());
    map.insert(s.to_owned(), leaked);
    leaked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom() -> Brand {
        Brand { app_name: "Acme PDF".into(), logo_light: None, logo_dark: None, mark: None, community_links: false, ..Brand::default() }
    }

    #[test]
    fn the_default_brand_is_pdfcraft_and_renames_nothing() {
        let b = Brand::default();
        assert_eq!(b.app_name, "PdfCraft");
        assert_eq!(b.storage_key, "pdfcraft");
        assert!(b.community_links);
        assert_eq!(b.logo_light.is_some() && b.logo_dark.is_some() && b.mark.is_some(), cfg!(feature = "artcraft-marks"));
        assert_eq!(b.renamed("Welcome to PdfCraft"), None);
        assert_eq!(b.window_title(Some("a.pdf")), "a.pdf — PdfCraft");
        assert_eq!(b.window_title(None), "PdfCraft");
    }

    #[test]
    fn a_custom_brand_renames_every_mention() {
        let b = custom();
        assert_eq!(b.renamed("Welcome to PdfCraft").as_deref(), Some("Welcome to Acme PDF"));
        assert_eq!(b.renamed("PdfCraft {v} is available. PdfCraft").as_deref(), Some("Acme PDF {v} is available. Acme PDF"));
        assert_eq!(b.renamed("Open a file"), None);
        assert_eq!(b.window_title(Some("a.pdf")), "a.pdf — Acme PDF");
    }

    #[test]
    fn rebrand_leaves_strings_without_the_name_alone() {
        // No brand is set in tests (it is process-wide), so this is the default path.
        let s = "Open a file";
        assert!(std::ptr::eq(rebrand(s), s));
        let named = "Welcome to PdfCraft";
        assert_eq!(rebrand(named), named);
    }
}
