// ВИДАТКОВА НАКЛАДНА — Typst шаблон
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
    ВИДАТКОВА НАКЛАДНА № #d.number від #d.date р.
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
    "ПОКУПЕЦЬ",
    client.name,
    client.edrpou,
    client.iban,
  ),
  reqs-cell(
    "ПОСТАЧАЛЬНИК",
    company.name,
    company.edrpou,
    company.iban,
    address: company.address,
  ),
)

#v(6mm)

// ── Таблиця позицій ──────────────────────────────────────────────────────
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

  // Заголовок
  th[№],
  th[Найменування товару / послуги],
  th[К-сть],
  th[Од.],
  th[Ціна, грн],
  th[Сума, грн],

  // Рядки
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
    #bold[Загальна сума:] #d.total грн
  ]
]

#if d.vat_amount != "" and d.vat_amount != "0.00" [
  #align(right)[
    #text(size: 10pt)[
      у т.ч. ПДВ: #d.vat_amount грн
    ]
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
  #text(size: 9pt)[#name] \
  #v(8mm)
  #line(length: 60mm, stroke: 0.5pt)
  #text(size: 8pt, fill: luma(120))[(підпис)]
]

#grid(
  columns: (1fr, 1fr),
  gutter:  10mm,
  sign-block("ПОКУПЕЦЬ (отримав)",     client.name),
  sign-block("ПОСТАЧАЛЬНИК (відпустив)", company.name),
)

#v(4mm)
#align(center)[
  #text(size: 8pt, fill: luma(140))[
    Документ складено у двох примірниках, що мають однакову юридичну силу.
  ]
]
