// build.rs — скрипт збірки, запускається перед компіляцією основного коду.
// slint_build::compile_with_config() читає .slint файли та генерує Rust код,
// який потім підключається через slint::include_modules!() у main.rs.
fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_include_paths(vec!["ui".into()]);
    slint_build::compile_with_config("ui/app.slint", config)
        .expect("Помилка компіляції Slint UI. Перевір ui/app.slint.");
}
