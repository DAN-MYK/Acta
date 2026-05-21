// АКТ КОРИГУВАННЯ — Typst шаблон
// Дані передаються через: --input 'data=<JSON рядок>'

#import sys: inputs

#let raw = inputs.at("data", default: "{}")
#let d = json(bytes(raw))

#let company = d.company
#let client  = d.client
#let items   = d.items

// ── Налаштування сторінки ─────────────────────────────────────────────────
#set page(
  paper:  "a4",
  margin: (top: 20mm, bottom: 20mm, left: 20mm, right: 20mm),
)

#set text(font: ("Libertinus Serif", "FreeSerif", "DejaVu Serif"), size: 10pt, lang: "uk")
#set par(justify: false)

// ── Допоміжні стилі ──────────────────────────────────────────────────────
#let label-style = text.with(size: 8pt, fill: luma(100))
#let value-style = text.with(size: 10pt)
#let bold        = text.with(weight: "bold")

// ── Заголовок ─────────────────────────────────────────────────────────────
#align(center)[
  #text(size: 13pt, weight: "bold")[
    АКТ КОРИГУВАННЯ № #d.number від #d.date р.
  ]
]

#v(2mm)

#align(center)[
  #text(size: 9pt, fill: luma(80))[
    до акту виконаних робіт № #d.original_act_number
  ]
]

#v(6mm)

// ── Реквізити (2 колонки) ─────────────────────────────────────────────────
#let reqs-cell(header, name, edrpou, iban, address: none) = [
  #block(
    stroke: 0.5pt + luma(160),
    inset:  (x: 5mm, y: 4mm),
    radius: 2pt,
    width:  100%,
  )[
    #bold[#header] \
    #v(1mm)
    #label-style[Найменування:] \
    #value-style[#name] \
    #v(1mm)
    #label-style[ЄДРПОУ/ІПН:] \
    #value-style[#edrpou] \
    #v(1mm)
    #label-style[IBAN:] \
    #value-style[#iban]
    #if address != none [
      #v(1mm)
      #label-style[Адреса:] \
      #value-style[#address]
    ]
  ]
]

#grid(
  columns: (1fr, 1fr),
  gutter: 5mm,
  reqs-cell(
    "ЗАМОВНИК",
    client.name,
    client.edrpou,
    client.iban,
  ),
  reqs-cell(
    "ВИКОНАВЕЦЬ",
    company.name,
    company.edrpou,
    company.iban,
    address: company.address,
  ),
)

#v(6mm)

// ── Таблиця позицій коригування ──────────────────────────────────────────
#let col-widths = (8mm, 1fr, 18mm, 18mm, 26mm, 26mm)
#let header-fill = luma(230)

#let th(content) = table.cell(fill: header-fill, align: center + horizon)[
  #text(weight: "bold", size: 9pt)[#content]
]

#let td(content, al: left) = table.cell(align: al + horizon)[
  #text(size: 9pt)[#content]
]

#table(
  columns:    col-widths,
  stroke:     0.5pt + luma(100),
  inset:      (x: 3mm, y: 2.5mm),

  th[№],
  th[Найменування роботи / послуги],
  th[К-сть],
  th[Од.],
  th[Ціна, грн],
  th[Сума, грн],

  ..items.map(it => (
    td(str(it.num),    al: center),
    td(it.name),
    td(it.qty,         al: center),
    td(it.unit,        al: center),
    td(it.price,       al: right),
    td(it.amount,      al: right),
  )).flatten(),
)

#v(4mm)

// ── Підсумок ──────────────────────────────────────────────────────────────
#align(right)[
  #text(size: 11pt)[
    #bold[Загальна сума коригування:] #d.total грн
  ]
]

#v(2mm)

#block(
  stroke:  0.5pt + luma(160),
  inset:   (x: 5mm, y: 3mm),
  radius:  2pt,
  width:   100%,
)[
  #label-style[Сума прописом:] \
  #value-style[#d.total_words]
]

#if d.notes != "" [
  #v(2mm)
  #block(
    stroke:  0.5pt + luma(200),
    inset:   (x: 5mm, y: 3mm),
    radius:  2pt,
    width:   100%,
  )[
    #label-style[Примітки:] \
    #value-style[#d.notes]
  ]
]

#v(8mm)

// ── Блок підписів ─────────────────────────────────────────────────────────
#let sign-block(role, name) = [
  #bold[#role] \
  #v(1mm)
  #grid(
    columns: (auto, 1fr),
    gutter: 5mm,
    [#name],
    [#line(length: 100%, stroke: 0.5pt + luma(150))],
  )
  #v(1mm)
  #label-style[(підпис)]
]

#grid(
  columns: (1fr, 1fr),
  gutter: 10mm,
  sign-block("Від замовника:", client.name),
  sign-block("Від виконавця:", company.name),
)
