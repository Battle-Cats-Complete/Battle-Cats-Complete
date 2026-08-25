use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use image::{ImageFormat, RgbaImage};
use tracing::error;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use nyanko::graphics::rig::SpriteSheet;

const EXPORT_DIR: &str = "exports";

pub struct Cut {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Cut {
    pub fn label(&self, index: usize) -> String {
        if self.name.is_empty() {
            return index.to_string();
        }

        format!("{} - {}", index, self.name)
    }

    fn stem(&self, index: usize) -> String {
        if self.name.is_empty() {
            return index.to_string();
        }

        format!("{}-{}", index, self.name)
    }
}

const PLACEHOLDER_NAME: &str = "-";

fn named(raw: String) -> String {
    if raw.trim() == PLACEHOLDER_NAME { String::new() } else { raw }
}

pub struct Sheet {
    pub stem: String,
    pub cuts: Vec<Cut>,
    image: RgbaImage,
}

impl Sheet {
    pub fn size(&self) -> (u32, u32) {
        (self.image.width(), self.image.height())
    }

    pub fn crop(&self, index: usize) -> Option<RgbaImage> {
        let cut = self.cuts.get(index)?;

        if cut.width == 0 || cut.height == 0 {
            return None;
        }

        let width = cut.width.min(self.image.width().saturating_sub(cut.x));
        let height = cut.height.min(self.image.height().saturating_sub(cut.y));

        if width == 0 || height == 0 {
            return None;
        }

        Some(image::imageops::crop_imm(&self.image, cut.x, cut.y, width, height).to_image())
    }
}

pub fn load(png: &Path, imgcut: &Path) -> Result<Sheet, String> {
    let png_bytes = fs::read(png).map_err(|error| format!("Could not read {}: {}", png.display(), error))?;
    let cut_bytes = fs::read(imgcut).map_err(|error| format!("Could not read {}: {}", imgcut.display(), error))?;

    let parsed = SpriteSheet::parse(&png_bytes, &cut_bytes).map_err(|error| format!("{}", error))?;

    let image = image::load_from_memory(&png_bytes)
        .map_err(|error| format!("Could not decode {}: {}", png.display(), error))?
        .to_rgba8();

    let cuts = parsed
        .cuts
        .into_iter()
        .map(|cut| Cut {
            name: named(cut.name),
            x: cut.x.max(0) as u32,
            y: cut.y.max(0) as u32,
            width: cut.width.max(0) as u32,
            height: cut.height.max(0) as u32,
        })
        .collect();

    Ok(Sheet { stem: stem_of(png), cuts, image })
}

pub fn partner(path: &Path, extension: &str) -> Option<PathBuf> {
    let candidate = path.with_extension(extension);

    candidate.is_file().then_some(candidate)
}

pub fn export_cut(sheet: &Sheet, index: usize) -> Result<PathBuf, String> {
    let cut = sheet.cuts.get(index).ok_or_else(|| "That cut is no longer available".to_string())?;
    let cropped = sheet.crop(index).ok_or_else(|| "That cut has no pixels to export".to_string())?;

    let directory = export_dir()?;
    let target = directory.join(format!("{}-{}.png", cut.stem(index), sheet.stem));

    cropped
        .save_with_format(&target, ImageFormat::Png)
        .map_err(|error| format!("Could not write {}: {}", target.display(), error))?;

    Ok(target)
}

pub fn export_all(sheet: &Sheet) -> Result<PathBuf, String> {
    let directory = export_dir()?;
    let target = directory.join(format!("{}.zip", sheet.stem));

    let file = fs::File::create(&target).map_err(|error| format!("Could not write {}: {}", target.display(), error))?;
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut written = 0usize;

    for (index, cut) in sheet.cuts.iter().enumerate() {
        let Some(cropped) = sheet.crop(index) else {
            continue;
        };

        let mut encoded = Vec::new();
        if let Err(error) = cropped.write_to(&mut std::io::Cursor::new(&mut encoded), ImageFormat::Png) {
            error!("Could not encode cut {} of {}: {}", index, sheet.stem, error);
            continue;
        }

        if let Err(error) = archive.start_file(format!("{}.png", cut.stem(index)), options) {
            error!("Could not start zip entry for cut {}: {}", index, error);
            continue;
        }

        if let Err(error) = archive.write_all(&encoded) {
            error!("Could not write zip entry for cut {}: {}", index, error);
            continue;
        }

        written += 1;
    }

    archive.finish().map_err(|error| format!("Could not finalize {}: {}", target.display(), error))?;

    if written == 0 {
        return Err("No cuts had any pixels to export".to_string());
    }

    Ok(target)
}

fn export_dir() -> Result<PathBuf, String> {
    let directory = PathBuf::from(EXPORT_DIR);

    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create {}: {}", directory.display(), error))?;

    Ok(directory)
}

fn stem_of(path: &Path) -> String {
    path.file_stem().map_or_else(|| "sheet".to_string(), |stem| stem.to_string_lossy().to_string())
}
