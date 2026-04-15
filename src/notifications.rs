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

/// Публічна точка входу — тикає кожні 60 секунд.
pub async fn reminder_loop(pool: Arc<PgPool>) {
    run_loop(pool, Duration::from_secs(60)).await;
}

/// Цикл нагадувань з конфігурованим інтервалом — `pub(crate)` для тестів.
///
/// На кожному тику запитує `tasks::due_reminders` і показує системне
/// сповіщення для кожного завдання, що настає. Помилки БД логуються
/// і не перерву цикл.
pub(crate) async fn run_loop(pool: Arc<PgPool>, period: Duration) {
    let mut ticker = interval(period);

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
    use std::sync::Arc;

    use chrono::{TimeZone, Utc};
    use sqlx::postgres::PgPoolOptions;
    use tokio::time::Duration;

    use super::{reminder_body, reminder_loop, run_loop};

    // ── Допоміжна функція ─────────────────────────────────────────────────────
    //
    // Lazy-пул до закритого порту.  ECONNREFUSED повертається ОС одразу,
    // без таймерів tokio → сумісний з `start_paused = true`.
    // `connect_lazy` не відкриває з'єднання при створенні — помилка виникає
    // тільки при першому запиті всередині циклу.
    fn fake_pool() -> sqlx::PgPool {
        PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_millis(50))
            .connect_lazy("postgres://x@127.0.0.1:54321/nonexistent")
            .expect("connect_lazy не повинен падати — підключення відкладено")
    }

    // ── reminder_body: чиста функція ─────────────────────────────────────────

    #[test]
    fn reminder_body_formats_datetime_for_user() {
        let dt = Utc
            .with_ymd_and_hms(2026, 4, 1, 15, 45, 0)
            .single()
            .expect("valid dt");
        assert_eq!(reminder_body(Some(dt)), "Нагадування на 01.04.2026 15:45");
    }

    #[test]
    fn reminder_body_has_fallback_text_when_date_missing() {
        assert_eq!(reminder_body(None), "Час перевірити завдання");
    }

    #[test]
    fn reminder_body_midnight_formats_correctly() {
        let dt = Utc
            .with_ymd_and_hms(2026, 12, 31, 0, 0, 0)
            .single()
            .expect("valid dt");
        assert_eq!(reminder_body(Some(dt)), "Нагадування на 31.12.2026 00:00");
    }

    // ── reminder_loop: скасування ─────────────────────────────────────────────
    //
    // Нескінченний цикл не можна «завершити» нормально — тільки через abort().
    // Перевіряємо що abort не призводить до паніки та повертає JoinError::cancelled.

    #[tokio::test]
    async fn reminder_loop_aborts_cleanly_before_first_tick() {
        // abort() до того як задача встигла виконати перший тік
        let handle = tokio::spawn(reminder_loop(Arc::new(fake_pool())));
        handle.abort();
        let err = handle.await.unwrap_err();
        assert!(err.is_cancelled(), "abort повинен скасувати, а не панікувати");
    }

    #[tokio::test]
    async fn reminder_loop_aborts_cleanly_while_waiting_for_next_tick() {
        // Перший тік — одразу; після помилки БД цикл чекає наступного тіку (3600s).
        // Скасовуємо поки він «сплить» між тіками.
        let handle = tokio::spawn(run_loop(Arc::new(fake_pool()), Duration::from_secs(3600)));
        tokio::time::sleep(Duration::from_millis(150)).await;
        handle.abort();
        let err = handle.await.unwrap_err();
        assert!(err.is_cancelled(), "abort між тіками повинен скасувати задачу");
    }

    // ── run_loop: стійкість до помилок БД ────────────────────────────────────
    //
    // Цикл не повинен завершуватись або панікувати через помилку з'єднання з БД.
    // ECONNREFUSED → `tracing::error!` → продовження циклу.

    #[tokio::test]
    async fn run_loop_survives_repeated_db_errors_without_panicking() {
        // Інтервал 50ms → за 400ms цикл зробить ~6 тіків, кожен з помилкою БД.
        let handle = tokio::spawn(run_loop(Arc::new(fake_pool()), Duration::from_millis(50)));
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert!(
            !handle.is_finished(),
            "run_loop не повинен завершуватись після серії помилок БД"
        );
        handle.abort();
    }

    // ── run_loop: поведінка таймера ────────────────────────────────────────────
    //
    // `start_paused = true` заморожує tokio-таймери.
    // advance(d) просуває годинник і відпускає всі таймери що «настали».
    // ECONNREFUSED не потребує tokio-таймера → сумісний з паузованим годинником.

    #[tokio::test(start_paused = true)]
    async fn run_loop_does_not_self_terminate_between_ticks() {
        let handle =
            tokio::spawn(run_loop(Arc::new(fake_pool()), Duration::from_secs(60)));

        // Перший тік: interval завжди спрацьовує одразу при першому poll().
        // Просуваємо 1ms щоб дати задачі час виконати тік і помилковий DB-запит.
        tokio::time::advance(Duration::from_millis(1)).await;
        for _ in 0..100 { tokio::task::yield_now().await; }
        assert!(!handle.is_finished(), "alive після тіку 1");

        // Другий тік: через 60s. Між тіками loop заблокований на ticker.tick().await.
        tokio::time::advance(Duration::from_secs(60)).await;
        for _ in 0..100 { tokio::task::yield_now().await; }
        assert!(!handle.is_finished(), "alive після тіку 2");

        // Третій тік: через ще 60s.
        tokio::time::advance(Duration::from_secs(60)).await;
        for _ in 0..100 { tokio::task::yield_now().await; }
        assert!(!handle.is_finished(), "alive після тіку 3");

        handle.abort();
        assert!(handle.await.unwrap_err().is_cancelled());
    }

    #[tokio::test(start_paused = true)]
    async fn run_loop_does_not_tick_before_interval_elapses() {
        // Перевіряємо що між тіками цикл заблокований, а не «крутиться» активно.
        // Якщо б цикл не чекав інтервал, він би продовжував робити DB-виклики
        // і міг би завершитись або панікувати — а він не повинен.
        let handle =
            tokio::spawn(run_loop(Arc::new(fake_pool()), Duration::from_secs(3600)));

        // Перший тік (одразу)
        tokio::time::advance(Duration::from_millis(1)).await;
        for _ in 0..100 { tokio::task::yield_now().await; }

        // Просуваємо лише половину інтервалу (1800s < 3600s) —
        // другий тік ще не повинен настати.
        tokio::time::advance(Duration::from_secs(1800)).await;
        for _ in 0..100 { tokio::task::yield_now().await; }

        assert!(
            !handle.is_finished(),
            "loop заблокований на ticker.tick().await, очікує решту інтервалу"
        );
        handle.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn run_loop_fires_tick_exactly_at_interval_boundary() {
        // Перевіряємо що тік ТАКИ спрацьовує коли інтервал минув.
        // До межі — живий та чекає; після повного advance — живий (не завершився).
        let handle =
            tokio::spawn(run_loop(Arc::new(fake_pool()), Duration::from_secs(60)));

        // Перший тік
        tokio::time::advance(Duration::from_millis(1)).await;
        for _ in 0..100 { tokio::task::yield_now().await; }

        // Рівно інтервал — другий тік повинен спрацювати
        tokio::time::advance(Duration::from_secs(60)).await;
        for _ in 0..100 { tokio::task::yield_now().await; }

        // Ще раз — третій тік
        tokio::time::advance(Duration::from_secs(60)).await;
        for _ in 0..100 { tokio::task::yield_now().await; }

        // Після трьох тіків з помилками БД loop все ще живий
        assert!(!handle.is_finished(), "loop живий після 3 тіків з помилками БД");
        handle.abort();
    }
}
