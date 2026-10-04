use std::ffi::c_char;

use plugin_interface::{RGBA_CHANNELS, parse_parameters, run_plugin, validate_rgba};

#[derive(Debug, PartialEq, Eq)]
struct MirrorParams {
    horizontal: bool,
    vertical: bool,
}

impl Default for MirrorParams {
    fn default() -> Self {
        Self {
            horizontal: true,
            vertical: false,
        }
    }
}

impl MirrorParams {
    fn parse(text: &str) -> Result<Self, String> {
        let mut params = Self::default();
        for (key, value) in parse_parameters(text)? {
            let target = match key {
                "horizontal" => &mut params.horizontal,
                "vertical" => &mut params.vertical,
                _ => return Err(format!("неизвестный параметр: {key}")),
            };
            *target = value
                .parse::<bool>()
                .map_err(|_| format!("{key} должен быть true или false"))?;
        }
        Ok(params)
    }
}

fn mirror(width: u32, height: u32, pixels: &mut [u8], params: &MirrorParams) -> Result<(), String> {
    validate_rgba(width, height, pixels)?;
    let (width, height) = (width as usize, height as usize);
    if params.horizontal {
        for y in 0..height {
            for x in 0..width / 2 {
                swap_pixels(pixels, y * width + x, y * width + width - 1 - x);
            }
        }
    }
    if params.vertical {
        for y in 0..height / 2 {
            for x in 0..width {
                swap_pixels(pixels, y * width + x, (height - 1 - y) * width + x);
            }
        }
    }
    Ok(())
}

fn swap_pixels(pixels: &mut [u8], first: usize, second: usize) {
    for channel in 0..RGBA_CHANNELS {
        pixels.swap(
            first * RGBA_CHANNELS + channel,
            second * RGBA_CHANNELS + channel,
        );
    }
}

/// Отражает RGBA-изображение на месте, не пропуская панику через границу FFI.
///
/// # Safety
/// Хост передаёт живой, исключительный RGBA-буфер из `width * height * 4` байт
/// и отдельную NUL-терминированную строку параметров. Указатели не сохраняются.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
) {
    // SAFETY: инварианты указателей обеспечивает хост, общий адаптер проверяет размеры.
    unsafe {
        run_plugin(
            "mirror",
            width,
            height,
            rgba_data,
            params,
            |width, height, pixels, text| {
                mirror(width, height, pixels, &MirrorParams::parse(text)?)
            },
        );
    }
}

const _: plugin_interface::ProcessImageFn = process_image;

#[cfg(test)]
mod tests {
    use super::*;

    fn original() -> Vec<u8> {
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    }

    #[test]
    fn parses_values_and_defaults() {
        assert_eq!(
            MirrorParams::parse("horizontal=false,vertical=true").unwrap(),
            MirrorParams {
                horizontal: false,
                vertical: true
            }
        );
        assert_eq!(
            MirrorParams::parse("horizontal=false").unwrap(),
            MirrorParams {
                horizontal: false,
                vertical: false
            }
        );
        assert_eq!(
            MirrorParams::parse("vertical=true").unwrap(),
            MirrorParams {
                horizontal: true,
                vertical: true
            }
        );
    }

    #[test]
    fn rejects_invalid_parameters() {
        for text in [
            "",
            "unknown=true",
            "horizontal=1",
            "vertical=maybe",
            "horizontal=true,horizontal=false",
            "horizontal",
        ] {
            assert!(MirrorParams::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn mirrors_2_by_2_horizontally() {
        let mut pixels = original();
        mirror(
            2,
            2,
            &mut pixels,
            &MirrorParams {
                horizontal: true,
                vertical: false,
            },
        )
        .unwrap();
        assert_eq!(
            pixels,
            [5, 6, 7, 8, 1, 2, 3, 4, 13, 14, 15, 16, 9, 10, 11, 12]
        );
    }

    #[test]
    fn mirrors_2_by_2_vertically() {
        let mut pixels = original();
        mirror(
            2,
            2,
            &mut pixels,
            &MirrorParams {
                horizontal: false,
                vertical: true,
            },
        )
        .unwrap();
        assert_eq!(
            pixels,
            [9, 10, 11, 12, 13, 14, 15, 16, 1, 2, 3, 4, 5, 6, 7, 8]
        );
    }

    #[test]
    fn mirrors_2_by_2_both_axes() {
        let mut pixels = original();
        mirror(
            2,
            2,
            &mut pixels,
            &MirrorParams {
                horizontal: true,
                vertical: true,
            },
        )
        .unwrap();
        assert_eq!(
            pixels,
            [13, 14, 15, 16, 9, 10, 11, 12, 5, 6, 7, 8, 1, 2, 3, 4]
        );
    }

    #[test]
    fn disabled_axes_and_single_pixel_are_unchanged() {
        let mut pixels = original();
        mirror(
            2,
            2,
            &mut pixels,
            &MirrorParams {
                horizontal: false,
                vertical: false,
            },
        )
        .unwrap();
        assert_eq!(pixels, original());
        let mut pixel = [1, 2, 3, 4];
        mirror(
            1,
            1,
            &mut pixel,
            &MirrorParams {
                horizontal: true,
                vertical: true,
            },
        )
        .unwrap();
        assert_eq!(pixel, [1, 2, 3, 4]);
    }

    #[test]
    fn invalid_buffer_is_rejected() {
        assert!(mirror(2, 2, &mut [0; 12], &MirrorParams::default()).is_err());
    }
}
