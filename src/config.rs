// Конфігурація програми — зберігається між сесіями у ~/.config/acta/config.toml
//
// Залежності (додати до Cargo.toml):
//   toml = "0.8"   — серіалізація/десеріалізація TOML
//   dirs = "5"     — крос-платформний шлях до конфіг-директорії
//
// На Windows: %APPDATA%\acta\config.toml (C:\Users\<user>\AppData\Roaming\acta\)
// На Linux/macOS: ~/.config/acta/config.toml

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Налаштування програми що зберігаються між запусками.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    /// UUID останньої активної компанії — автоматично обирається при наступному запуску.
    pub last_company_id: Option<Uuid>,
}

impl AppConfig {
    /// Завантажити конфігурацію з файлу.
    /// Якщо файл відсутній або пошкоджений — повертає Default (порожній конфіг).
    pub fn load() -> Self {
        let path = config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Зберегти конфігурацію у файл.
    /// Помилки ігноруються (запис конфігу — некритична операція).
    pub fn save(&self) {
        let path = config_path();
        // Створюємо директорію якщо вона ще не існує
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = toml::to_string(self) {
            let _ = std::fs::write(path, s);
        }
    }
}

/// Повертає шлях до файлу конфігурації.
///
/// `dirs::config_dir()` — крос-платформна функція:
///   - Windows: %APPDATA% (C:\Users\<user>\AppData\Roaming)
///   - macOS: ~/Library/Application Support
///   - Linux: ~/.config
fn config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("acta")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_company() {
        let cfg = AppConfig::default();
        assert!(cfg.last_company_id.is_none());
    }

    #[test]
    fn config_path_ends_with_expected_suffix() {
        let path = config_path();
        assert!(path.ends_with("acta/config.toml") || path.ends_with("acta\\config.toml"));
    }
}
