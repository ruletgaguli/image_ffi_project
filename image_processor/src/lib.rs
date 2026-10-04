pub mod error;
pub mod plugin_loader;

use std::{
    ffi::CString,
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use image::{ImageFormat, ImageReader, RgbaImage};
use plugin_interface::validate_rgba;

use error::{ProcessorError, Result};
use plugin_loader::Plugin;

#[derive(Debug, Parser)]
#[command(
    name = "image_processor",
    about = "Обработка изображения динамическим плагином",
    arg_required_else_help = true
)]
pub struct Config {
    /// Исходное изображение.
    #[arg(long)]
    pub input: PathBuf,
    /// Выходной файл в формате PNG.
    #[arg(long)]
    pub output: PathBuf,
    /// Имя плагина без префикса, расширения и пути: mirror или blur.
    #[arg(long)]
    pub plugin: String,
    /// UTF-8-файл с параметрами обработки.
    #[arg(long)]
    pub params: PathBuf,
    /// Каталог с динамическими библиотеками.
    #[arg(long, default_value = "target/debug")]
    pub plugin_path: PathBuf,
}

pub(crate) fn ensure_file(path: &Path, kind: &'static str) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(ProcessorError::MissingFile {
            kind,
            path: path.to_path_buf(),
        })
    }
}

/// Загружает изображение и параметры, вызывает плагин и сохраняет PNG.
///
/// # Safety
/// Выбранная библиотека должна быть доверенной и соблюдать контракт Plugin::load.
/// Проверить соответствие произвольной библиотеки C ABI во время выполнения нельзя.
pub unsafe fn process(config: &Config) -> Result<()> {
    ensure_file(&config.input, "входное изображение")?;
    ensure_file(&config.params, "файл параметров")?;
    let text = fs::read_to_string(&config.params).map_err(|source| ProcessorError::Io {
        operation: "не удалось прочитать параметры в UTF-8",
        path: config.params.clone(),
        source,
    })?;
    let params =
        CString::new(text).map_err(|error| ProcessorError::ParamsNul(error.nul_position()))?;
    let reader = ImageReader::open(&config.input)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|source| ProcessorError::Io {
            operation: "не удалось прочитать изображение",
            path: config.input.clone(),
            source,
        })?;
    let image = reader
        .decode()
        .map_err(|source| ProcessorError::Image {
            path: config.input.clone(),
            source,
        })?
        .into_rgba8();
    let (width, height) = image.dimensions();
    let mut pixels = image.into_raw();
    log::debug!("изображение {}x{}, плагин {}", width, height, config.plugin);
    // SAFETY: контракт process требует доверенный плагин с согласованной сигнатурой.
    let plugin = unsafe { Plugin::load(&config.plugin_path, &config.plugin) }?;
    plugin.process(width, height, &mut pixels, &params)?;
    validate_rgba(width, height, &pixels).map_err(ProcessorError::InvalidBuffer)?;
    let image = RgbaImage::from_raw(width, height, pixels).ok_or_else(|| {
        ProcessorError::InvalidBuffer("не удалось восстановить изображение".into())
    })?;
    image
        .save_with_format(&config.output, ImageFormat::Png)
        .map_err(|source| ProcessorError::Image {
            path: config.output.clone(),
            source,
        })?;
    Ok(())
}
