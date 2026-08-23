// Step 6.5 (docs/phase-6-reporting.md §6.5): voucher print layout.
// Server-side code/asset only -- there is no endpoint anywhere in this
// codebase that lets an end user load or edit this file (B23 fix; contrast
// the legacy's FastReport preview exposing pbEdit/pbLoad to every user who
// can open any preview, 04-06-a.md §6.2).
//
// Reproduces 04-06-b.md §6.10's checklist for this document: A4 portrait,
// 10mm margins, RTL column order, zero renders blank, no negative sign, the
// amount-in-words footer, and the (up to four) configurable signature
// blocks -- fed in as plain strings by api/src/pdf.rs, never hard-coded
// here, so a real "signature block" is just data, not a template edit.

#import sys: inputs
#let d = inputs

#set page(paper: "a4", margin: (x: 10mm, y: 12mm), footer: context [
  #set text(size: 8pt, dir: ltr)
  #align(center)[
    #str(counter(page).get().first()) از #str(counter(page).final().first())
  ]
])
#set text(font: "Vazirmatn", size: 10pt, dir: rtl, lang: "fa")

#align(center)[
  #text(size: 14pt, weight: "bold")[#d.header.organization_name]
  #linebreak()
  #text(size: 11pt)[#d.header.fiscal_year_caption]
  #linebreak()
  #text(size: 12pt, weight: "bold")[#d.header.report_title]
]
#v(4mm)
#line(length: 100%, stroke: 0.5pt)
#v(2mm)

#grid(
  columns: (1fr, 1fr, 1fr),
  align: (right, right, right),
  [شماره سند: #d.voucher_number], [تاریخ: #d.voucher_date], [شرح: #d.description],
)
#v(3mm)

#table(
  columns: (1fr, 3fr, 2fr, 2fr),
  align: (right, right, left, left),
  stroke: 0.5pt,
  table.header([حساب], [شرح ردیف], [بدهکار], [بستانکار]),
  ..d.lines.map(l => (l.account_label, l.description, l.debit, l.credit)).flatten(),
  table.cell(colspan: 2, align: right)[جمع کل],
  [#d.total_debit], [#d.total_credit],
)

#v(4mm)
#text(size: 9pt)[مبلغ به حروف: #d.amount_in_words]

#v(10mm)
#if d.header.signature_labels.len() > 0 [
  #grid(
    columns: (1fr,) * d.header.signature_labels.len(),
    ..d.header.signature_labels.map(s => align(center)[#s]),
  )
]
