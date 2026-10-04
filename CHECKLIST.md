# Проверка требований проектной работы

| Требование | Реализация | Проверка |
| --- | --- | --- |
| Workspace, CLI и два cdylib | Корневой Cargo.toml, image_processor, mirror_plugin, blur_plugin | `cargo build --workspace` |
| Четыре обязательных флага, каталог по умолчанию | image_processor/src/lib.rs, Config | `--help`, тест no_arguments_show_help |
| Чтение файла параметров, UTF-8, CString | image_processor/src/lib.rs | тесты отсутствующего файла и NUL-байта |
| Декодирование изображения, RGBA8, сохранение PNG | image_processor/src/lib.rs | реальные CLI-сценарии, verify_results |
| Определение входного формата по содержимому | ImageReader::with_guessed_format | CLI-тест с PNG, сохранённым как input.bin |
| Имя библиотеки по правилам ОС | image_processor/src/plugin_loader.rs | uses_platform_library_name |
| Точная сигнатура process_image | plugin_interface/src/lib.rs, экспорты обоих плагинов | проверка типов при компиляции, реальные вызовы |
| Library и CString живут до возврата | Plugin владеет Library, Symbol заимствует её; CString в process | локальная проверка unsafe-кода, CLI-сценарии |
| Проверка длины и переполнения RGBA | plugin_interface::rgba_len, validate_rgba | dimensions_and_lengths_are_checked |
| Плагин не освобождает и не сохраняет указатели | run_plugin использует локальные CStr и срез | локальная проверка unsafe-кода |
| catch_unwind внутри плагина, сохранение исходного буфера | общий адаптер run_plugin | errors_and_panics_leave_host_buffer_unchanged |
| Параметры обоих плагинов и значения по умолчанию | MirrorParams::parse, BlurParams::parse | parses_values_and_defaults, rejects_invalid_parameters |
| Неверные параметры не изменяют буфер через настоящий FFI | run_plugin, экспорт обоих cdylib | verify_plugin_contract |
| Пользовательский каталог плагинов | Config::plugin_path, Plugin::load | verify_plugin_contract |
| Отражение по горизонтали, вертикали и обеим осям | mirror_plugin/src/lib.rs | три побайтовых теста 2x2, verify_results |
| Размытие однородного буфера и яркого центра | blur_plugin/src/lib.rs | uniform_buffer_is_unchanged, bright_center_spreads_to_neighbors |
| Ошибки файла, изображения, библиотеки и символа | ProcessorError, CLI-тесты | cargo test, дополнительные реальные ошибочные запуски |
| Минимальный unsafe, комментарии инвариантов | plugin_loader, run_plugin, экспорты | Clippy, локальная проверка unsafe-кода |
| Тестовые PNG и параметры в репозитории | input.png, params/mirror.txt, params/blur.txt | запуск команд из README |
| Документация сборки и запуска | README.md | воспроизведение команд из README |

GitHub Actions повторяет локальные проверки на Linux. Локальная сборка на macOS не подтверждает выполнение на Windows: для этого нужно отдельно собрать и запустить workspace на Windows.
