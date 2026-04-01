use std::sync::Arc;

use chrono::{DateTime, Utc};
use notify_rust::{Notification, Timeout};
use sqlx::PgPool;
use tokio::time::{Duration, interval};

use crate::db::tasks;

fn reminder_body(reminder_at: Option<DateTime<Utc>>) -> String {
    reminder_at
        .map(|dt| format!("Нагадування на {}", dt.format("%d.%m.%Y %H:%M")))
        .unwrap_or_else(|| "Час перевірити завдання".to_string())
}

pub async fn reminder_loop(pool: Arc<PgPool>) {
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        ticker.tick().await;

        match tasks::due_reminders(&pool).await {
            Ok(due_tasks) => {
                for task in due_tasks {
                    let body = reminder_body(task.reminder_at);

                    let _ = Notification::new()
                        .appname("Acta")
                        .summary(task.title)
                        .body(&body)
                        .timeout(Timeout::Milliseconds(8_000))
                        .show();
                }
            }
            Err(error) => {
                tracing::error!("Помилка циклу нагадувань: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::reminder_body;

    #[test]
    fn reminder_body_formats_datetime_for_user() {
        let dt = Utc
            .with_ymd_and_hms(2026, 4, 1, 15, 45, 0)
            .single()
            .expect("valid dt");
        let body = reminder_body(Some(dt));
        assert_eq!(body, "Нагадування на 01.04.2026 15:45");
    }

    #[test]
    fn reminder_body_has_fallback_text_when_date_missing() {
        let body = reminder_body(None);
        assert_eq!(body, "Час перевірити завдання");
    }
}
