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
        Self::load_from(&config_path())
    }

    /// Зберегти конфігурацію у файл.
    /// Помилки ігноруються (запис конфігу — некритична операція).
    pub fn save(&self) {
        self.save_to(&config_path());
    }

    fn load_from(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save_to(&self, path: &std::path::Path) {
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

    #[test]
    fn save_and_load_roundtrip() {
        let id = Uuid::new_v4();
        let cfg = AppConfig { last_company_id: Some(id) };

        let path = std::env::temp_dir().join(format!("acta_config_test_{}.toml", id));
        cfg.save_to(&path);
        let loaded = AppConfig::load_from(&path);
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.last_company_id, Some(id));
    }

    #[test]
    fn load_from_missing_file_returns_default() {
        let path = std::env::temp_dir().join("acta_config_nonexistent_xyz.toml");
        let cfg = AppConfig::load_from(&path);
        assert!(cfg.last_company_id.is_none());
    }

    #[test]
    fn load_from_corrupt_file_returns_default() {
        let path = std::env::temp_dir().join("acta_config_corrupt_xyz.toml");
        let _ = std::fs::write(&path, b"not valid toml !!!@@@");
        let cfg = AppConfig::load_from(&path);
        let _ = std::fs::remove_file(&path);
        assert!(cfg.last_company_id.is_none());
    }
}
