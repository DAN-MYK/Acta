# Acta Icons

Канонічна папка для всіх SVG-іконок фронтенду Acta.

## Що тут лежить

- окремі `.svg` файли для кожної іконки;
- [_template.svg](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/icons/_template.svg) як starter template;
- [index.ts](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/icons/index.ts) як реєстр іконок для `AppIcon`.

## Швидкі правила

- `viewBox="0 0 24 24"`
- `fill="none"` на кореневому `svg`
- `stroke="currentColor"`
- базовий `stroke-width="1.8"`
- переважно `stroke-linecap="round"` і `stroke-linejoin="round"`
- без жорстко вшитих кольорів
- не малювати фон або контейнер усередині самого SVG

## Робочі розміри в UI

- `14` - дрібні індикатори
- `15-16` - кнопки, рядки, бейджі
- `18-20` - секції, навігація
- `24+` - preview або галерея

SVG завжди лишається `24x24`, а масштабування робиться через [AppIcon.svelte](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/components/AppIcon.svelte).

## Як додати нову іконку

1. Скопіюй [_template.svg](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/icons/_template.svg) у новий файл.
2. Намалюй форму в поточному стилі Acta.
3. Додай імпорт у [index.ts](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/icons/index.ts).
4. Якщо іконка доменна або статусна, додай мапінг у [frontend/src/lib/icon-maps.ts](/C:/Users/MykhailoDan/apps/Acta/frontend/src/lib/icon-maps.ts).
5. Використай її через `AppIcon`.

## Повний style guide

Детальний документ лежить тут:

[docs/architecture/icon-style-guide.md](/C:/Users/MykhailoDan/apps/Acta/docs/architecture/icon-style-guide.md)
