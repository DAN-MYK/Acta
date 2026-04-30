use anyhow::Result;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::app_ctx::{AppCtx, AppScreen};
use crate::models::{NewTask, TaskPriority, TaskStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboxAction {
    CreateOverdueReminder(Uuid),
    ReconcilePayment(Uuid),
    OpenDocument(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboxEffect {
    RefreshScreens(Vec<AppScreen>),
    OpenDocument {
        screen: AppScreen,
        document_id: String,
    },
}

pub fn parse_inbox_action(id: &str, kind: &str) -> Option<InboxAction> {
    match kind {
        "overdue" => prefixed_uuid(id, "act:").map(InboxAction::CreateOverdueReminder),
        "unmatched" => prefixed_uuid(id, "pay:").map(InboxAction::ReconcilePayment),
        "unsigned" | "act-needed" => Some(InboxAction::OpenDocument(id.to_string())),
        _ => None,
    }
}

pub async fn execute_inbox_action(
    ctx: &AppCtx,
    id: &str,
    kind: &str,
) -> Result<Option<InboxEffect>> {
    let Some(action) = parse_inbox_action(id, kind) else {
        return Ok(None);
    };

    let effect = match action {
        InboxAction::CreateOverdueReminder(act_id) => {
            create_overdue_act_reminder(ctx, act_id).await?;
            InboxEffect::RefreshScreens(vec![AppScreen::Dashboard, AppScreen::Tasks])
        }
        InboxAction::ReconcilePayment(payment_id) => {
            crate::db::payments::mark_reconciled(ctx.pool(), payment_id).await?;
            InboxEffect::RefreshScreens(vec![AppScreen::Dashboard, AppScreen::Payments])
        }
        InboxAction::OpenDocument(document_id) => InboxEffect::OpenDocument {
            screen: AppScreen::Documents,
            document_id,
        },
    };

    Ok(Some(effect))
}

fn prefixed_uuid(id: &str, prefix: &str) -> Option<Uuid> {
    id.strip_prefix(prefix)
        .and_then(|value| Uuid::parse_str(value).ok())
}

async fn create_overdue_act_reminder(ctx: &AppCtx, act_id: Uuid) -> Result<()> {
    let Some((act, _items)) = crate::db::acts::get_by_id(ctx.pool(), act_id).await? else {
        tracing::warn!("inbox_action: act {act_id} не знайдено для нагадування");
        return Ok(());
    };

    let title = format!("Нагадати про оплату акту {}", act.number);
    let already_open = crate::db::tasks::list_by_act(ctx.pool(), act_id)
        .await?
        .into_iter()
        .any(|task| {
            matches!(task.status, TaskStatus::Open | TaskStatus::InProgress) && task.title == title
        });

    if already_open {
        tracing::info!("inbox_action: задача-нагадування для акту {act_id} вже існує");
        return Ok(());
    }

    let reminder_at = Utc::now() + Duration::hours(2);
    let task = NewTask {
        title,
        description: Some("Створено з Inbox на головному екрані.".to_string()),
        priority: TaskPriority::High,
        due_date: Some(reminder_at),
        reminder_at: Some(reminder_at),
        counterparty_id: None,
        act_id: Some(act_id),
    };

    crate::db::tasks::create(ctx.pool(), ctx.company_id(), &task).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{parse_inbox_action, InboxAction};

    #[test]
    fn parses_supported_inbox_actions() {
        let act_id = Uuid::new_v4();
        let pay_id = Uuid::new_v4();

        assert_eq!(
            parse_inbox_action(&format!("act:{act_id}"), "overdue"),
            Some(InboxAction::CreateOverdueReminder(act_id))
        );
        assert_eq!(
            parse_inbox_action(&format!("pay:{pay_id}"), "unmatched"),
            Some(InboxAction::ReconcilePayment(pay_id))
        );
        assert_eq!(
            parse_inbox_action("act:doc-1", "unsigned"),
            Some(InboxAction::OpenDocument("act:doc-1".to_string()))
        );
    }

    #[test]
    fn rejects_invalid_inbox_payloads() {
        assert_eq!(parse_inbox_action("act:missing-uuid", "overdue"), None);
        assert_eq!(parse_inbox_action("whatever", "unknown"), None);
    }
}
