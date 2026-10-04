use std::{
    ffi::CStr,
    path::{Path, PathBuf},
};

use libloading::{Library, Symbol};
use plugin_interface::{PROCESS_IMAGE_SYMBOL, ProcessImageFn, validate_rgba};

use crate::{
    ensure_file,
    error::{ProcessorError, Result},
};

/// Формирует путь к библиотеке по правилам ОС, не принимая пути в имени плагина.
pub fn plugin_filename(name: &str) -> Result<PathBuf> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(ProcessorError::InvalidPluginName(name.to_string()));
    }
    Ok(libloading::library_filename(name).into())
}

pub struct Plugin {
    library: Library,
    path: PathBuf,
}

impl Plugin {
    /// Загружает доверенную библиотеку и сохраняет её до завершения всех вызовов.
    ///
    /// # Safety
    /// Библиотека должна экспортировать process_image с точной сигнатурой ProcessImageFn.
    /// Она не должна сохранять или освобождать указатели хоста, выходить за границы
    /// буфера или допускать панику через C ABI. Её инициализаторы и деструкторы также
    /// должны быть безопасны для вызывающего процесса.
    pub unsafe fn load(directory: &Path, name: &str) -> Result<Self> {
        let path = directory.join(plugin_filename(name)?);
        ensure_file(&path, "библиотека плагина")?;
        let absolute = path.canonicalize().map_err(|source| ProcessorError::Io {
            operation: "не удалось определить путь к библиотеке",
            path: path.clone(),
            source,
        })?;
        // SAFETY: вызывающий разрешает выполнение кода доверенного плагина и гарантирует его ABI.
        let library =
            unsafe { Library::new(absolute) }.map_err(|source| ProcessorError::PluginLoad {
                path: path.clone(),
                source,
            })?;
        Ok(Self { library, path })
    }

    /// Передаёт плагину живой буфер и C-строку, проверив размеры перед вызовом.
    pub fn process(&self, width: u32, height: u32, pixels: &mut [u8], params: &CStr) -> Result<()> {
        validate_rgba(width, height, pixels).map_err(ProcessorError::InvalidBuffer)?;
        // SAFETY: контракт load гарантирует точную сигнатуру; Symbol заимствует живую Library.
        let process: Symbol<'_, ProcessImageFn> = unsafe { self.library.get(PROCESS_IMAGE_SYMBOL) }
            .map_err(|source| ProcessorError::PluginSymbol {
                path: self.path.clone(),
                source,
            })?;
        // SAFETY: размеры проверены, буфер исключительный, params и Library живут до возврата.
        unsafe { process(width, height, pixels.as_mut_ptr(), params.as_ptr()) };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_platform_library_name() {
        let expected = format!(
            "{}mirror{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        );
        assert_eq!(plugin_filename("mirror").unwrap(), PathBuf::from(expected));
    }

    #[test]
    fn rejects_paths_and_extensions() {
        for name in [
            "",
            ".",
            "../mirror",
            "plugins/mirror",
            "plugins\\mirror",
            "mirror.so",
            "mirror.dll",
            "mirror dylib",
        ] {
            assert!(plugin_filename(name).is_err(), "{name}");
        }
    }
}
