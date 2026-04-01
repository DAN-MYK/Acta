// build.rs — скрипт збірки, запускається перед компіляцією основного коду.
// slint_build::compile() читає .slint файли та генерує Rust код,
// який потім підключається через slint::include_modules!() у main.rs.
fn main() {
    slint_build::compile("ui/main.slint")
        .expect("Помилка компіляції Slint UI. Перевір ui/main.slint.");
}
