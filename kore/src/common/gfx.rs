use image::imageops;
use image::RgbaImage;

pub fn autocrop(img: RgbaImage) -> RgbaImage {
    let (width, height) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0, 0);
    let mut found = false;

    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] > 0 {
            if x < min_x { min_x = x; }
            if x > max_x { max_x = x; }
            if y < min_y { min_y = y; }
            if y > max_y { max_y = y; }
            found = true;
        }
    }
    if !found { return img; }
    imageops::crop_imm(&img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}