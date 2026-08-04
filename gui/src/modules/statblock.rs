pub(crate) mod builder;
mod draw;

use image::RgbaImage;

#[derive(Clone)]
pub enum JobResult {
    Copy(Result<RgbaImage, String>),
    Save(Result<(), String>),
}
