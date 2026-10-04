use image::{ImageFormat, Rgba, RgbaImage};

fn main() -> Result<(), image::ImageError> {
    let mut image = RgbaImage::from_pixel(160, 112, Rgba([238, 243, 247, 255]));
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        if (x / 8 + y / 8) % 2 == 0 {
            *pixel = Rgba([220, 229, 235, 255]);
        }
        if (12..64).contains(&x) && (12..76).contains(&y) {
            *pixel = Rgba([10, 158, 142, 255]);
        }
        let (dx, dy) = (i64::from(x) - 112, i64::from(y) - 44);
        if dx * dx + dy * dy <= 25 * 25 {
            *pixel = Rgba([234, 82, 100, 255]);
        }
        if (20..46).contains(&x) && (24..40).contains(&y) {
            *pixel = Rgba([247, 214, 48, 255]);
        }
        if (88..148).contains(&x) && (86..98).contains(&y) {
            *pixel = Rgba([75, 73, 183, 255]);
        }
    }
    image.save_with_format("input.png", ImageFormat::Png)
}
