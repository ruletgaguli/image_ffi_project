use std::process::ExitCode;

use clap::Parser;
use image_processor::{Config, process};

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let config = Config::parse();
    // SAFETY: пользователь выбирает доверенный плагин; встроенные плагины workspace соблюдают ABI.
    match unsafe { process(&config) } {
        Ok(()) => {
            println!("Результат сохранён: {}", config.output.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Ошибка: {error}");
            ExitCode::FAILURE
        }
    }
}
