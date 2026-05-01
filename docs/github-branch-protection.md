# Захист гілки `main`

Цей репозиторій має використовувати branch protection для `main`, щоб Tauri cutover не ламався через випадковий merge без потрібних CI-перевірок.

## Required checks

Для `main` потрібно зробити обов'язковими такі status checks:

- `Frontend build and checks`
- `Rust backend legacy safety net`
- `SQLx offline metadata contract`
- `Tauri migration compile gate`
- `Postgres integration and Tauri vertical slice`

`Tauri Release Gates / Windows Tauri bundle gate` не варто робити required для PR merge. Це slow gate для `main` і ручного release-контролю, а не для швидкого feedback loop у pull request.

## Що ще має бути увімкнено

- Require a pull request before merging
- Require approvals: `1`
- Dismiss stale pull request approvals when new commits are pushed
- Require conversation resolution before merging
- Require branches to be up to date before merging
- Do not allow force pushes
- Do not allow deletions
- Include administrators

## Автоматичне застосування через `gh`

Після `gh auth login` можна застосувати налаштування командою:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .github/scripts/set-main-branch-protection.ps1
```

Якщо репозиторій або owner інші:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .github/scripts/set-main-branch-protection.ps1 -Owner "<owner>" -Repo "<repo>" -Branch "main"
```

## Навіщо це потрібно

- merge у `main` не пройде, якщо зламано Tauri compile path
- SQLx offline cache стає формальним контрактом, а не доброю волею
- vertical slice через Postgres і `tauri_vertical_slice` стає обов'язковим cutover-gate
- повільний Windows bundle лишається окремою страховкою й не сповільнює кожен PR
