use std::ffi::c_char;

use plugin_interface::{RGBA_CHANNELS, parse_parameters, run_plugin, validate_rgba};

#[derive(Debug, PartialEq, Eq)]
struct BlurParams {
    radius: u32,
    iterations: u32,
}

impl Default for BlurParams {
    fn default() -> Self {
        Self {
            radius: 1,
            iterations: 1,
        }
    }
}

impl BlurParams {
    fn parse(text: &str) -> Result<Self, String> {
        let mut params = Self::default();
        for (key, value) in parse_parameters(text)? {
            let target = match key {
                "radius" => &mut params.radius,
                "iterations" => &mut params.iterations,
                _ => return Err(format!("неизвестный параметр: {key}")),
            };
            *target = value
                .parse::<u32>()
                .map_err(|_| format!("{key} должен быть целым числом от 0 до {}", u32::MAX))?;
        }
        Ok(params)
    }
}

fn blur(width: u32, height: u32, pixels: &mut [u8], params: &BlurParams) -> Result<(), String> {
    validate_rgba(width, height, pixels)?;
    if params.radius == 0 || params.iterations == 0 {
        return Ok(());
    }
    let (width, height) = (width as usize, height as usize);
    let radius = usize::try_from(params.radius)
        .map_err(|_| "радиус слишком велик для этой платформы".to_string())?;
    let mut scratch = Vec::new();
    scratch
        .try_reserve_exact(pixels.len())
        .map_err(|error| format!("не удалось выделить буфер размытия: {error}"))?;
    scratch.resize(pixels.len(), 0);
    for _ in 0..params.iterations {
        for y in 0..height {
            blur_line(
                pixels,
                &mut scratch,
                y * width * RGBA_CHANNELS,
                RGBA_CHANNELS,
                width,
                radius,
            );
        }
        for x in 0..width {
            blur_line(
                &scratch,
                pixels,
                x * RGBA_CHANNELS,
                width * RGBA_CHANNELS,
                height,
                radius,
            );
        }
    }
    Ok(())
}

// Скользящая сумма позволяет размывать каждую строку за линейное время.
fn blur_line(
    source: &[u8],
    target: &mut [u8],
    start: usize,
    stride: usize,
    len: usize,
    radius: usize,
) {
    let radius = radius.min(len - 1);
    let mut sum = [0_u64; RGBA_CHANNELS];
    for index in 0..=radius {
        for channel in 0..RGBA_CHANNELS {
            sum[channel] += u64::from(source[start + index * stride + channel]);
        }
    }
    for position in 0..len {
        let left = position.saturating_sub(radius);
        let right = position.saturating_add(radius).min(len - 1);
        let count = (right - left + 1) as u64;
        for channel in 0..RGBA_CHANNELS {
            target[start + position * stride + channel] = (sum[channel] / count) as u8;
        }
        if position >= radius {
            let leaving = position - radius;
            for channel in 0..RGBA_CHANNELS {
                sum[channel] -= u64::from(source[start + leaving * stride + channel]);
            }
        }
        let entering = position.saturating_add(radius).saturating_add(1);
        if entering < len {
            for channel in 0..RGBA_CHANNELS {
                sum[channel] += u64::from(source[start + entering * stride + channel]);
            }
        }
    }
}

/// Размывает RGBA-изображение, сохраняя исходный буфер при ошибке или панике.
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
    // SAFETY: хост обеспечивает валидность памяти; run_plugin изолирует обработку и паники.
    unsafe {
        run_plugin(
            "blur",
            width,
            height,
            rgba_data,
            params,
            |width, height, pixels, text| blur(width, height, pixels, &BlurParams::parse(text)?),
        );
    }
}

const _: plugin_interface::ProcessImageFn = process_image;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_values_and_defaults() {
        assert_eq!(
            BlurParams::parse("radius=4,iterations=3").unwrap(),
            BlurParams {
                radius: 4,
                iterations: 3
            }
        );
        assert_eq!(
            BlurParams::parse("radius=2").unwrap(),
            BlurParams {
                radius: 2,
                iterations: 1
            }
        );
        assert_eq!(
            BlurParams::parse("iterations=2").unwrap(),
            BlurParams {
                radius: 1,
                iterations: 2
            }
        );
    }

    #[test]
    fn rejects_invalid_parameters() {
        for text in [
            "",
            "unknown=1",
            "radius=many",
            "iterations=1.5",
            "radius=-1",
            "radius=4294967296",
            "iterations=",
            "radius=1,radius=2",
        ] {
            assert!(BlurParams::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn uniform_buffer_is_unchanged() {
        let mut pixels = [25, 50, 75, 120].repeat(12);
        let original = pixels.clone();
        blur(
            4,
            3,
            &mut pixels,
            &BlurParams {
                radius: 4,
                iterations: 3,
            },
        )
        .unwrap();
        assert_eq!(pixels, original);
    }

    #[test]
    fn bright_center_spreads_to_neighbors() {
        let mut pixels = vec![0; 3 * 3 * RGBA_CHANNELS];
        for pixel in pixels.as_chunks_mut::<RGBA_CHANNELS>().0 {
            pixel[3] = 255;
        }
        pixels[4 * RGBA_CHANNELS] = 255;
        blur(3, 3, &mut pixels, &BlurParams::default()).unwrap();
        assert_eq!(
            pixels
                .iter()
                .step_by(RGBA_CHANNELS)
                .copied()
                .collect::<Vec<_>>(),
            [63, 42, 63, 42, 28, 42, 63, 42, 63]
        );
        assert!(
            pixels
                .as_chunks::<RGBA_CHANNELS>()
                .0
                .iter()
                .all(|pixel| pixel[3] == 255)
        );
    }

    #[test]
    fn zero_parameters_and_single_pixel_are_unchanged() {
        for params in [
            BlurParams {
                radius: 0,
                iterations: 3,
            },
            BlurParams {
                radius: 4,
                iterations: 0,
            },
        ] {
            let mut pixels = [1, 2, 3, 4].repeat(4);
            let original = pixels.clone();
            blur(2, 2, &mut pixels, &params).unwrap();
            assert_eq!(pixels, original);
        }
        let mut pixel = [10, 20, 30, 40];
        blur(
            1,
            1,
            &mut pixel,
            &BlurParams {
                radius: u32::MAX,
                iterations: 2,
            },
        )
        .unwrap();
        assert_eq!(pixel, [10, 20, 30, 40]);
    }

    #[test]
    fn invalid_buffer_is_rejected() {
        assert!(blur(3, 3, &mut [0; 4], &BlurParams::default()).is_err());
    }

    #[test]
    fn sliding_windows_match_direct_two_pass_average() {
        for (width, height) in [(1_usize, 5_usize), (5, 1), (2, 3), (5, 4)] {
            for radius in [1, 2, 10] {
                let mut pixels: Vec<_> = (0..width * height * 4)
                    .map(|index| ((index * 37 + 11) % 256) as u8)
                    .collect();
                let original = pixels.clone();
                let mut horizontal = pixels.clone();
                let mut expected = pixels.clone();
                for y in 0..height {
                    for x in 0..width {
                        let left = x.saturating_sub(radius);
                        let right = (x + radius).min(width - 1);
                        for channel in 0..RGBA_CHANNELS {
                            let sum: u64 = (left..=right)
                                .map(|column| {
                                    u64::from(original[(y * width + column) * 4 + channel])
                                })
                                .sum();
                            horizontal[(y * width + x) * 4 + channel] =
                                (sum / (right - left + 1) as u64) as u8;
                        }
                    }
                }
                for y in 0..height {
                    for x in 0..width {
                        let top = y.saturating_sub(radius);
                        let bottom = (y + radius).min(height - 1);
                        for channel in 0..RGBA_CHANNELS {
                            let sum: u64 = (top..=bottom)
                                .map(|row| u64::from(horizontal[(row * width + x) * 4 + channel]))
                                .sum();
                            expected[(y * width + x) * 4 + channel] =
                                (sum / (bottom - top + 1) as u64) as u8;
                        }
                    }
                }
                blur(
                    width as u32,
                    height as u32,
                    &mut pixels,
                    &BlurParams {
                        radius: radius as u32,
                        iterations: 1,
                    },
                )
                .unwrap();
                assert_eq!(pixels, expected, "{width}x{height}, radius={radius}");
            }
        }
    }
}
