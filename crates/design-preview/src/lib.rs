//! A saved design's picture and summary, read for file browsers without the kernel.
//!
//! Finder icons and Quick Look (apps/solidcraft-quicklook) show a `.sc` design without opening it:
//! they read the picture a desktop save keeps at `cache/thumbnail.png` and a short summary of
//! `document.json` (name, units, features, parameters). Nothing here evaluates, meshes or draws
//! the model, and nothing depends on the kernel, so an extension that links this crate stays
//! small and quick (docs/design-container.md, "Portable thumbnail").
//!
//! Input is hostile: a file browser hands over whatever file has the extension. Every read is
//! bounded, and any problem (not a container, a missing or oversized part, a malformed picture or
//! document) is `None`: the file browser then shows its generic icon or card.

use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use serde::Deserialize;

mod card;

pub use card::card;

/// The container formats this reader knows (docs/design-container.md; the second is the
/// pre-rename draft, docs/compatibility.md).
const CONTAINERS: &[&str] = &["solidcraft-container/1", "solvecraft-container/1"];
/// The largest design file looked at, as the app's own cap on a design (`io::max_design`).
const MAX_FILE: u64 = 2 << 30;
/// More entries than any design the app writes (docs/design-container.md, "Bounds").
const MAX_ENTRIES: usize = 10_000;
const MAX_CONTAINER_JSON: u64 = 4 << 10;
/// The saved picture's own bound is 256 KiB; a later, larger picture still fits.
const MAX_PICTURE: u64 = 8 << 20;
/// A picture wider or taller than this is not one SolidCraft wrote.
const MAX_PICTURE_SIDE: u32 = 4096;
/// The document metadata's cap for cache-like reads (`DesignContainerReader::preview`).
const MAX_DOCUMENT: u64 = 16 << 20;
/// A summary lists at most this many features and parameters; the rest are counted.
pub const MAX_LISTED: usize = 200;

/// The saved picture: PNG bytes and their size in pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// A parameter as the design stores it: its name and expression (`F_d1`, `60 mm`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub expression: String,
}

/// What a file browser shows about a design besides its picture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    /// The design's name, as it is in the app's title bar.
    pub name: String,
    /// The design's length unit (`mm`, `in`, …).
    pub units: String,
    /// The timeline's feature names in order, at most [`MAX_LISTED`].
    pub features: Vec<String>,
    /// How many features the timeline has in all.
    pub feature_count: usize,
    /// The user's parameters, at most [`MAX_LISTED`] (model parameters are left out).
    pub parameters: Vec<Parameter>,
    /// How many user parameters the design has in all.
    pub parameter_count: usize,
}

/// The picture a desktop save keeps in the design at `path`, if it has a valid one.
pub fn picture(path: &Path) -> Option<Picture> {
    let mut archive = open(path)?;
    let png = part(&mut archive, "cache/thumbnail.png", MAX_PICTURE)?;
    let (width, height) = png_size(&png)?;
    Some(Picture { png, width, height })
}

/// The name, units, features and parameters of the design at `path`.
pub fn summary(path: &Path) -> Option<Summary> {
    let mut archive = open(path)?;
    let document = part(&mut archive, "document.json", MAX_DOCUMENT)?;
    summarize(&document)
}

/// The summary of a `document.json`.
pub fn summarize(document: &[u8]) -> Option<Summary> {
    let doc: Document = serde_json::from_slice(document).ok()?;
    let user_parameters = doc.params.into_iter().filter(|p| !p.model);
    let mut parameters = Vec::new();
    let mut parameter_count = 0;
    for p in user_parameters {
        parameter_count += 1;
        if parameters.len() < MAX_LISTED {
            parameters.push(Parameter { name: p.name, expression: p.expr });
        }
    }
    let feature_count = doc.features.len();
    let features = doc.features.into_iter().take(MAX_LISTED).map(|f| f.name).collect();
    Some(Summary { name: doc.name, units: doc.units, features, feature_count, parameters, parameter_count })
}

/// The width and height a PNG's header gives, if the bytes start as a PNG SolidCraft could write.
pub fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    if png.get(..8)? != b"\x89PNG\r\n\x1a\n" || png.get(12..16)? != b"IHDR" {
        return None;
    }
    let side = |at: usize| png.get(at..at + 4).and_then(|b| b.try_into().ok()).map(u32::from_be_bytes);
    let (width, height) = (side(16)?, side(20)?);
    let fits = |s: u32| (1..=MAX_PICTURE_SIDE).contains(&s);
    (fits(width) && fits(height)).then_some((width, height))
}

impl Picture {
    /// The largest size with the picture's proportions that fits in `width` x `height` (a file
    /// browser's icon or card, in any unit), or `None` when that box is empty.
    pub fn fit(&self, width: f64, height: f64) -> Option<(f64, f64)> {
        let usable = |side: f64| side.is_finite() && side > 0.0;
        if !usable(width) || !usable(height) {
            return None;
        }
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        let scale = (width / w).min(height / h);
        Some((w * scale, h * scale))
    }
}

/// The ZIP container at `path`, after its size, entry count and `container.json` are checked.
fn open(path: &Path) -> Option<zip::ZipArchive<File>> {
    let file = File::open(path).ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE {
        return None;
    }
    let mut archive = zip::ZipArchive::new(file).ok()?;
    if archive.len() > MAX_ENTRIES {
        return None;
    }
    let container: Container = serde_json::from_slice(&part(&mut archive, "container.json", MAX_CONTAINER_JSON)?).ok()?;
    CONTAINERS.contains(&container.format.as_str()).then_some(archive)
}

/// One entry's bytes, read in full only when its declared and actual sizes are within `limit`.
fn part<R: Read + Seek>(archive: &mut zip::ZipArchive<R>, name: &str, limit: u64) -> Option<Vec<u8>> {
    let entry = archive.by_name(name).ok()?;
    if entry.size() > limit {
        return None;
    }
    let mut bytes = Vec::with_capacity(usize::try_from(entry.size()).ok()?);
    // One byte past the limit tells a lying header from an honest one.
    entry.take(limit + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

#[derive(Deserialize)]
struct Container {
    format: String,
}

/// The parts of `document.json` (crates/doc/src/format.rs) a summary reads; the rest is skipped.
#[derive(Deserialize)]
struct Document {
    #[serde(default)]
    name: String,
    #[serde(default)]
    units: String,
    #[serde(default)]
    features: Vec<Feature>,
    #[serde(default)]
    params: Vec<Param>,
}

#[derive(Deserialize)]
struct Feature {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct Param {
    #[serde(default)]
    name: String,
    #[serde(default)]
    expr: String,
    #[serde(default)]
    model: bool,
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
