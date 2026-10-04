use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("{kind} не найден или не является файлом: {path}")]
    MissingFile { kind: &'static str, path: PathBuf },
    #[error("{operation} ({path}): {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("ошибка изображения ({path}): {source}")]
    Image {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
    #[error(
        "некорректное имя плагина '{0}': нужны только латинские буквы, цифры, '_' и '-', без пути и расширения"
    )]
    InvalidPluginName(String),
    #[error("не удалось загрузить библиотеку {path}: {source}")]
    PluginLoad {
        path: PathBuf,
        #[source]
        source: libloading::Error,
    },
    #[error("не удалось найти process_image в {path}: {source}")]
    PluginSymbol {
        path: PathBuf,
        #[source]
        source: libloading::Error,
    },
    #[error("файл параметров содержит NUL-байт на позиции {0}; его нельзя передать как C-строку")]
    ParamsNul(usize),
    #[error("некорректный RGBA-буфер: {0}")]
    InvalidBuffer(String),
}

pub type Result<T> = std::result::Result<T, ProcessorError>;
