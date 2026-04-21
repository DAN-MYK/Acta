pub fn apply_settings_to_ui(ui: &crate::AppWindow) {
    ui.set_company_info(crate::CompanyInfo {
        full_name: slint::SharedString::default(),
        short_name: slint::SharedString::default(),
        edrpou: slint::SharedString::default(),
        ipn: slint::SharedString::default(),
        address: slint::SharedString::default(),
        director: slint::SharedString::default(),
        iban: slint::SharedString::default(),
        bank: slint::SharedString::default(),
        vat_registered: false,
        vat_cert: slint::SharedString::default(),
    });
}
