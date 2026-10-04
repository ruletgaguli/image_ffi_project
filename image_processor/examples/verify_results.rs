use image::RgbaImage;

fn variation(image: &RgbaImage) -> u64 {
    let mut sum = 0;
    for y in 0..image.height() {
        for x in 1..image.width() {
            for channel in 0..3 {
                sum += u64::from(
                    image.get_pixel(x - 1, y)[channel].abs_diff(image.get_pixel(x, y)[channel]),
                );
            }
        }
    }
    sum
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = image::open("input.png")?.into_rgba8();
    let mirror = image::open("out_mirror.png")?.into_rgba8();
    let blur = image::open("out_blur.png")?.into_rgba8();
    assert_eq!(mirror.dimensions(), input.dimensions());
    assert_eq!(blur.dimensions(), input.dimensions());
    for y in 0..input.height() {
        for x in 0..input.width() {
            assert_eq!(
                mirror.get_pixel(x, y),
                input.get_pixel(input.width() - 1 - x, y)
            );
        }
    }
    assert_ne!(blur.as_raw(), input.as_raw());
    assert!(variation(&blur) < variation(&input));
    println!(
        "Проверено: mirror отражает каждый пиксель, blur уменьшает резкие переходы, размеры сохранены."
    );
    Ok(())
}
