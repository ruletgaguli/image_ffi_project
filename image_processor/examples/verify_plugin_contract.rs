use std::{fs, path::Path, process::Command};

use image::{ImageFormat, RgbaImage};
use image_processor::plugin_loader::plugin_filename;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    let target = executable
        .parent()
        .and_then(Path::parent)
        .ok_or("не удалось найти каталог сборки")?;
    let cli = target.join(format!("image_processor{}", std::env::consts::EXE_SUFFIX));
    let directory = tempfile::tempdir()?;
    let plugins = directory.path().join("plugins");
    fs::create_dir(&plugins)?;
    for name in ["mirror", "blur"] {
        let filename = plugin_filename(name)?;
        fs::copy(target.join(&filename), plugins.join(filename))?;
    }
    let input = directory.path().join("input.png");
    let params = directory.path().join("params.txt");
    let output = directory.path().join("output.png");
    let pixels = vec![
        10, 20, 30, 255, 50, 60, 70, 255, 90, 100, 110, 255, 130, 140, 150, 255,
    ];
    let image = RgbaImage::from_raw(2, 2, pixels.clone()).ok_or("неверный тестовый буфер")?;
    image.save_with_format(&input, ImageFormat::Png)?;
    for (plugin, text, expected) in [
        (
            "mirror",
            "horizontal=true,vertical=true",
            vec![
                130, 140, 150, 255, 90, 100, 110, 255, 50, 60, 70, 255, 10, 20, 30, 255,
            ],
        ),
        ("blur", "radius=1,iterations=1", [70, 80, 90, 255].repeat(4)),
    ] {
        fs::write(&params, text)?;
        let result = run(&cli, &input, &output, &params, &plugins, plugin)?;
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(image::open(&output)?.into_rgba8().into_raw(), expected);
        for invalid in ["", "unknown=1", "horizontal=maybe", "radius=many"] {
            fs::write(&params, invalid)?;
            let result = run(&cli, &input, &output, &params, &plugins, plugin)?;
            assert!(
                result.status.success(),
                "API возвращает void: хост должен сохранить исходное изображение"
            );
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(stderr.contains(&format!("Плагин {plugin}:")), "{stderr}");
            assert!(!stderr.contains("panicked"), "{stderr}");
            assert_eq!(image::open(&output)?.into_rgba8().into_raw(), pixels);
        }
    }
    println!(
        "Проверено: оба плагина загружаются из --plugin-path, корректные параметры работают, некорректные не изменяют буфер."
    );
    Ok(())
}

fn run(
    cli: &Path,
    input: &Path,
    output: &Path,
    params: &Path,
    plugins: &Path,
    plugin: &str,
) -> std::io::Result<std::process::Output> {
    Command::new(cli)
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .arg("--params")
        .arg(params)
        .arg("--plugin-path")
        .arg(plugins)
        .args(["--plugin", plugin])
        .output()
}
