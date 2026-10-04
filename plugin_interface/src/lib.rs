use std::{
    collections::BTreeMap,
    ffi::{CStr, c_char},
    io::{self, Write},
    panic::{AssertUnwindSafe, catch_unwind},
};

pub const RGBA_CHANNELS: usize = 4;
pub const PROCESS_IMAGE_SYMBOL: &[u8] = b"process_image\0";
pub type ProcessImageFn = unsafe extern "C" fn(u32, u32, *mut u8, *const c_char);

/// Возвращает длину непустого RGBA-буфера с проверкой переполнения.
pub fn rgba_len(width: u32, height: u32) -> Result<usize, String> {
    if width == 0 || height == 0 {
        return Err("ширина и высота должны быть положительными".into());
    }
    let len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(RGBA_CHANNELS))
        .filter(|&len| len <= isize::MAX as usize)
        .ok_or_else(|| "размер RGBA-буфера слишком велик".to_string())?;
    Ok(len)
}

/// Проверяет соответствие размеров длине безопасного среза.
pub fn validate_rgba(width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
    let expected = rgba_len(width, height)?;
    if pixels.len() != expected {
        return Err(format!(
            "ожидалось {expected} байт RGBA, получено {}",
            pixels.len()
        ));
    }
    Ok(())
}

/// Разбирает пары ключ=значение, разделённые запятыми или переводами строк.
pub fn parse_parameters(text: &str) -> Result<BTreeMap<&str, &str>, String> {
    let mut values = BTreeMap::new();
    for entry in text.split([',', '\n']).map(str::trim) {
        if entry.is_empty() {
            continue;
        }
        let (key, value) = entry
            .split_once('=')
            .ok_or_else(|| format!("ожидалась пара ключ=значение: {entry}"))?;
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() || value.is_empty() {
            return Err("ключ и значение не должны быть пустыми".into());
        }
        if values.insert(key, value).is_some() {
            return Err(format!("параметр {key} указан повторно"));
        }
    }
    if values.is_empty() {
        return Err("строка параметров пуста".into());
    }
    Ok(values)
}

/// Выполняет обработку на копии и записывает результат только после успеха.
///
/// # Safety
/// Нулевые указатели отклоняются без разыменования. Ненулевой `rgba_data`
/// должен указывать на доступный для чтения и записи буфер из
/// `width * height * 4` байт. Доступ к нему должен быть исключительным.
/// Ненулевой `params` должен указывать на живую NUL-терминированную строку, которая
/// не пересекается с буфером изображения. Оба указателя живут до возврата.
pub unsafe fn run_plugin<F>(
    name: &str,
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
    process: F,
) where
    F: FnOnce(u32, u32, &mut [u8], &str) -> Result<(), String>,
{
    // Изменения рабочей копии при ошибке или панике не затрагивают буфер хоста.
    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        let len = rgba_len(width, height)?;
        if rgba_data.is_null() || params.is_null() {
            return Err("передан нулевой указатель".into());
        }
        // SAFETY: вызывающий гарантирует живую NUL-терминированную строку.
        let params = unsafe { CStr::from_ptr(params) }
            .to_str()
            .map_err(|_| "параметры должны быть в UTF-8".to_string())?;
        // SAFETY: длина проверена; хост гарантирует размер, время жизни и исключительный доступ.
        let pixels = unsafe { std::slice::from_raw_parts_mut(rgba_data, len) };
        let mut working = Vec::new();
        working
            .try_reserve_exact(len)
            .map_err(|error| format!("не удалось выделить рабочий буфер: {error}"))?;
        working.extend_from_slice(pixels);
        process(width, height, &mut working, params)?;
        pixels.copy_from_slice(&working);
        Ok(())
    }));
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(error)) => report_error(name, &error),
        Err(_) => report_error(name, "паника перехвачена; изображение не изменено"),
    }
}

fn report_error(name: &str, message: &str) {
    let _ = writeln!(io::stderr().lock(), "Плагин {name}: {message}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn dimensions_and_lengths_are_checked() {
        assert_eq!(rgba_len(2, 2).unwrap(), 16);
        assert!(rgba_len(0, 2).is_err());
        assert!(rgba_len(u32::MAX, u32::MAX).is_err());
        assert!(validate_rgba(2, 2, &[0; 12]).is_err());
    }

    #[test]
    fn parameters_reject_duplicates_and_missing_values() {
        assert!(parse_parameters("radius=1,radius=2").is_err());
        assert!(parse_parameters("radius=").is_err());
        assert!(parse_parameters("radius").is_err());
        assert!(parse_parameters(" , \n ").is_err());
        assert_eq!(
            parse_parameters(" radius = 2\niterations=3 ").unwrap()["radius"],
            "2"
        );
    }

    #[test]
    fn errors_and_panics_leave_host_buffer_unchanged() {
        let params = CString::new("value=true").unwrap();
        let original = [10, 20, 30, 255];
        let mut pixels = original;
        // SAFETY: массив и CString живут до конца вызова, размеры соответствуют массиву.
        unsafe {
            run_plugin(
                "test",
                1,
                1,
                pixels.as_mut_ptr(),
                params.as_ptr(),
                |_, _, pixels, _| {
                    pixels[0] = 99;
                    Err("ошибка обработки".into())
                },
            );
        }
        assert_eq!(pixels, original);
        // SAFETY: те же валидные указатели; паника остаётся внутри run_plugin.
        unsafe {
            run_plugin(
                "test",
                1,
                1,
                pixels.as_mut_ptr(),
                params.as_ptr(),
                |_, _, pixels, _| {
                    pixels[0] = 99;
                    panic!("проверка отката");
                },
            );
        }
        assert_eq!(pixels, original);
    }

    #[test]
    fn successful_processing_is_committed() {
        let params = CString::new("value=true").unwrap();
        let mut pixels = [10, 20, 30, 255];
        // SAFETY: указатели валидны и соответствуют одному RGBA-пикселю.
        unsafe {
            run_plugin(
                "test",
                1,
                1,
                pixels.as_mut_ptr(),
                params.as_ptr(),
                |_, _, pixels, _| {
                    pixels[0] = 99;
                    Ok(())
                },
            );
        }
        assert_eq!(pixels, [99, 20, 30, 255]);
    }

    #[test]
    fn null_pointers_are_rejected_without_dereference() {
        // SAFETY: run_plugin явно отклоняет null до создания среза или CStr.
        unsafe {
            run_plugin(
                "test",
                1,
                1,
                std::ptr::null_mut(),
                std::ptr::null(),
                |_, _, _, _| {
                    panic!("обработка не должна вызываться");
                },
            );
        }
    }
}
