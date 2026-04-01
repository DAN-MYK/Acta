// PDF генерація через Typst CLI
pub mod generator;

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::generator::amount_to_words;

    #[test]
    fn pdf_module_exports_generator_api() {
        let text = amount_to_words(&dec!(0.00));
        assert_eq!(text, "нуль гривень 00 копійок");
    }
}
