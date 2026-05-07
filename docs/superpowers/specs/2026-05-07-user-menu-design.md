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
      <AppIcon name="theme" size={14} />
      <span>Темна тема</span>
      <span class="user-menu-toggle" class:on={$settings.screen?.preferences.darkMode ?? false}></span>
    </button>

    <button
      role="menuitem"
      class="user-menu-item user-menu-item-danger"
      on:click={() => { /* TODO: logout */ showUserMenu = false; }}
    >
      <AppIcon name="open-link" size={14} />
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

---

## Що **не** змінюється

- `SettingsScreen.svelte` — повний екран налаштувань залишається
- Логіка `palette` — тільки додається `showUserMenu = false` при відкритті
- Tauri backend — не зачіпається

---

## Вийти — stub

Функціональність виходу наразі не реалізована в backend. Кнопка `Вийти` → `console.warn("TODO: logout")` + `showUserMenu = false`. Реалізується окремо.
