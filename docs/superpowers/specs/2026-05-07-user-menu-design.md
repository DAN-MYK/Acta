# Spec: User Menu (···) — dropdown у user-footer

**Дата:** 2026-05-07  
**Статус:** Готово до реалізації

---

## Контекст

У `frontend/src/App.svelte` є `<button class="user-more">···</button>` без `on:click`. Поруч — окрема кнопка шестерні для переходу в Налаштування. Завдання: зробити `···` функціональним і прибрати шестерню.

---

## Що змінюється

### 1. Видалити кнопку шестерні

Прибрати блок `.nav-item-settings` (рядки ~334–345 у `App.svelte`) з sidebar.  
`Ctrl+7` шорткат до `settings` у `handleKeydown` — **залишається**.

### 2. Додати змінну стану

```svelte
let showUserMenu = false;
```

### 3. Кнопка `···`

```svelte
<button
  class="user-more"
  type="button"
  aria-label="Меню користувача"
  aria-haspopup="menu"
  aria-expanded={showUserMenu}
  on:click={() => {
    showUserMenu = !showUserMenu;
    if (showUserMenu) palette.close();
  }}
>···</button>
```

### 4. Mutual exclusion — palette

У місці де викликається `palette.toggle()` додати `showUserMenu = false`:

```ts
palette.toggle();
showUserMenu = false;
```

### 5. Escape — пріоритет

У `handleKeydown`, блок `Escape`:

```ts
if (event.key === "Escape") {
  if (showUserMenu) {
    event.preventDefault();
    showUserMenu = false;
    return;
  }
  if ($palette.open) {
    event.preventDefault();
    closePalette();
    return;
  }
}
```

### 6. Dropdown + backdrop (всередині `.user-footer`)

`.user-footer` отримує `position: relative`.

Розмітка dropdown після `.user-more`:

```svelte
{#if showUserMenu}
  <button
    type="button"
    class="user-menu-backdrop"
    aria-label="Закрити меню"
    on:click={() => showUserMenu = false}
  ></button>

  <div class="user-menu" role="menu">
    <button
      role="menuitem"
      class="user-menu-item"
      on:click={() => { navigation.go("settings"); showUserMenu = false; }}
    >
      <AppIcon name="settings" size={14} />
      <span>Налаштування</span>
    </button>

    <button
      role="menuitemcheckbox"
      class="user-menu-item"
      aria-checked={$settings.screen?.preferences.darkMode ?? false}
      on:click={onUserMenuToggleTheme}
    >
      <AppIcon name="appearance" size={14} />
      <span>Темна тема</span>
      <span class="user-menu-toggle" class:on={$settings.screen?.preferences.darkMode ?? false}></span>
    </button>

    <button
      role="menuitem"
      class="user-menu-item user-menu-item-danger"
      on:click={() => { /* TODO: logout */ showUserMenu = false; }}
    >
      <AppIcon name="openLink" size={14} />
      <span>Вийти</span>
    </button>
  </div>
{/if}
```

### 7. Логіка toggle теми

```ts
async function onUserMenuToggleTheme() {
  const darkMode = !($settings.screen?.preferences.darkMode ?? false);
  theme.setMode(darkMode ? "dark" : "light");
  settings.updatePreference("darkMode", darkMode);
  await settings.savePreferences();
  await appShell.reloadShellChrome();
}
```

Меню **не закривається** після toggle — користувач може перемикати і бачити результат.

---

## CSS

### `.user-footer`
Додати `position: relative`.

### Backdrop
```css
.user-menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
  background: transparent;
  border: none;
  cursor: default;
  padding: 0;
}
```

### Dropdown
```css
.user-menu {
  position: absolute;
  bottom: calc(100% + 6px);
  right: 0;
  z-index: 50;
  min-width: 178px;
  background: var(--acta-color-bg-elevated);
  border: 1px solid var(--acta-color-border);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.10);
  overflow: hidden;
}
```

> **Увага:** `.sidebar` має `overflow-x: hidden` — box-shadow може бути обрізана. Перевірити в UI. Якщо обрізається — замінити `box-shadow` на `filter: drop-shadow(0 4px 16px rgba(0,0,0,.10))` на `.user-menu`.

### Пункти меню
```css
.user-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 12px;
  font: inherit;
  font-size: 12.5px;
  text-align: left;
  border: none;
  border-bottom: 1px solid var(--acta-color-bg-subtle);
  background: transparent;
  color: var(--acta-color-text);
  cursor: pointer;
}
.user-menu-item:last-child { border-bottom: none; }
.user-menu-item:hover { background: var(--acta-color-bg-hover); }
.user-menu-item-danger { color: var(--acta-color-danger, #C0392B); }
```

### Toggle switch
```css
.user-menu-toggle {
  margin-left: auto;
  width: 28px;
  height: 16px;
  border-radius: 8px;
  background: var(--acta-color-border-strong);
  position: relative;
  flex-shrink: 0;
  transition: background 160ms ease;
}
.user-menu-toggle::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #fff;
  transition: transform 160ms ease;
}
.user-menu-toggle.on {
  background: var(--acta-color-primary, #3B6EF0);
}
.user-menu-toggle.on::after {
  transform: translateX(12px);
}
```

---

## Файли що змінюються

| Файл | Що змінюється |
|---|---|
| `frontend/src/App.svelte` | Видалення `.nav-item-settings`, додавання state + handlers + dropdown markup |
| `frontend/src/styles.css` | Нові класи: `.user-menu`, `.user-menu-backdrop`, `.user-menu-item`, `.user-menu-toggle`; `position: relative` для `.user-footer` |
| `frontend/src/__tests__/AppShell.test.ts` | Оновити regex на рядку 219, додати нові тести |

---

## Тести

### Оновити наявний тест (AppShell.test.ts, рядок 219)

CSS-правило `@media (max-width: 720px)` після видалення `.nav-item-settings` виглядатиме як `.nav-item { font-size: 13px }` без `.nav-item-settings`. Regex треба оновити:

```ts
// було:
expect(styles).toMatch(/@media\s*\(max-width:\s*720px\)[\s\S]*\.nav-item,\s*\.nav-item-settings\s*\{[\s\S]*font-size:\s*13px/);

// стане:
expect(styles).toMatch(/@media\s*\(max-width:\s*720px\)[\s\S]*\.nav-item\s*\{[\s\S]*font-size:\s*13px/);
```

> Якщо CSS-правило залишить `.nav-item-settings` у селекторі (не видаляючи його там), то тест не треба міняти — але тоді клас `.nav-item-settings` вже не існує в HTML, що семантично неправильно. Рекомендується прибрати клас із CSS-правила разом з HTML.

### Нові тести

Додати до `AppShell.test.ts` або окремий `UserMenu.test.ts`:

| Тест | Що перевіряє |
|---|---|
| Відкриття меню | Клік на `.user-more` → `[role="menu"]` з'являється |
| Закриття через backdrop | Клік на `.user-menu-backdrop` → меню зникає |
| Escape закриває меню | `keydown Escape` при відкритому меню → `showUserMenu = false` |
| Escape не закриває palette якщо меню відкрите | Пріоритет: меню закривається, palette залишається |
| Ctrl+K закриває меню | `keydown Ctrl+K` при відкритому меню → `showUserMenu = false`, palette відкривається |
| Toggle теми | Клік на пункт теми → `themeStore.setMode` викликається з правильним значенням |
| Навігація в settings | Клік на «Налаштування» → `navigation.go("settings")` + меню закривається |

---

## Mobile / compact поведінка

`.user-footer` вже прихований на `max-width: 980px` (CSS: `display: none`). Це означає: на вузьких вікнах кнопка `···` недоступна, і після видалення шестерні **Налаштування** не матимуть видимої точки входу в sidebar.

**Рішення для v1:** приймається як є. Проект — desktop-first Tauri, вікно < 980px не є штатним сценарієм. Доступ до налаштувань залишається через:
- `Ctrl+7` (клавіатурний шорткат)
- Command Palette (`Ctrl+K` → "Налаштування")

**Spec фіксує це явно:** compact-режим не отримує окремої точки входу до налаштувань у v1. Якщо в майбутньому з'явиться mobile/compact layout — треба окремо передбачити доступ (наприклад, додати settings до bottom nav).

---

## Що **не** змінюється

- `SettingsScreen.svelte` — повний екран налаштувань залишається
- Логіка `palette` — тільки додається `showUserMenu = false` при відкритті
- Tauri backend — не зачіпається

---

## Вийти — stub

Функціональність виходу наразі не реалізована в backend. Кнопка `Вийти` → `console.warn("TODO: logout")` + `showUserMenu = false`. Реалізується окремо.
