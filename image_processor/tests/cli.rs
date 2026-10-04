use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use image::{ImageFormat, Rgba, RgbaImage};
use image_processor::plugin_loader::plugin_filename;

fn run(input: &Path, params: &Path, plugins: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_image_processor"))
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .args(["--plugin", "mirror"])
        .arg("--params")
        .arg(params)
        .arg("--plugin-path")
        .arg(plugins)
        .output()
        .unwrap()
}

fn assert_failure(result: Output, expected: &str, output: &Path) {
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains(expected), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert!(!output.exists());
}

#[test]
fn no_arguments_show_help() {
    let result = Command::new(env!("CARGO_BIN_EXE_image_processor"))
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&result.stderr);
    assert!(text.contains("--input"));
    assert!(text.contains("--output"));
    assert!(text.contains("--plugin-path"));
}

#[test]
fn missing_input_and_parameter_files_are_reported() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.png");
    let params = directory.path().join("params.txt");
    let output = directory.path().join("output.png");
    assert_failure(
        run(&input, &params, directory.path(), &output),
        "входное изображение не найден",
        &output,
    );
    fs::write(&input, b"placeholder").unwrap();
    assert_failure(
        run(&input, &params, directory.path(), &output),
        "файл параметров не найден",
        &output,
    );
}

#[test]
fn damaged_image_is_reported_without_loading_plugin() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.png");
    let params = directory.path().join("params.txt");
    let output = directory.path().join("output.png");
    fs::write(&input, b"not a PNG").unwrap();
    fs::write(&params, "horizontal=true").unwrap();
    assert_failure(
        run(&input, &params, directory.path(), &output),
        "ошибка изображения",
        &output,
    );
}

#[test]
fn missing_and_damaged_libraries_are_reported() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.bin");
    let params = directory.path().join("params.txt");
    let output = directory.path().join("output.png");
    RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255]))
        .save_with_format(&input, ImageFormat::Png)
        .unwrap();
    fs::write(&params, "horizontal=true").unwrap();
    assert_failure(
        run(&input, &params, directory.path(), &output),
        "библиотека плагина не найден",
        &output,
    );
    fs::write(
        directory.path().join(plugin_filename("mirror").unwrap()),
        b"not a library",
    )
    .unwrap();
    assert_failure(
        run(&input, &params, directory.path(), &output),
        "не удалось загрузить библиотеку",
        &output,
    );
}

#[test]
fn library_without_required_symbol_is_reported() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.png");
    let params = directory.path().join("params.txt");
    let output = directory.path().join("output.png");
    RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255]))
        .save_with_format(&input, ImageFormat::Png)
        .unwrap();
    fs::write(&params, "horizontal=true").unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/missing_symbol.rs");
    let compilation = Command::new("rustc")
        .arg(fixture)
        .args([
            "--edition=2024",
            "--crate-type=cdylib",
            "--crate-name=missing_symbol",
            "-o",
        ])
        .arg(directory.path().join(plugin_filename("mirror").unwrap()))
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    assert_failure(
        run(&input, &params, directory.path(), &output),
        "не удалось найти process_image",
        &output,
    );
}

#[test]
fn embedded_nul_in_parameters_is_reported() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.png");
    let params = directory.path().join("params.txt");
    let output = directory.path().join("output.png");
    fs::write(&input, b"placeholder").unwrap();
    fs::write(&params, b"horizontal=true\0vertical=false").unwrap();
    assert_failure(
        run(&input, &params, directory.path(), &output),
        "NUL-байт",
        &output,
    );
}
