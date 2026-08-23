// Step 6.5 (docs/phase-6-reporting.md §6.5): the generic tabular-report
// layout -- shared by every report in specs/04-reporting/04-02..04-04.md
// that renders as columns + a footer total (trial balances now; ledgers,
// the 6-column trial balance and the party-balance list are the same shape
// and can reuse this template unmodified once their own PDF routes are
// wired, per this step's own scope note in api/src/pdf.rs). Server-side
// asset only, no end-user edit surface (B23).
//
// 04-06-b.md §6.10's checklist: A4 portrait, 10mm margin, repeating column
// header on every page, grand-total footer, no group bands/forced breaks,
// RTL column order (columns are supplied already in right-to-left display
// order by the caller), zero -> blank, no negative sign.

#import sys: inputs
#let d = inputs

#set page(paper: "a4", margin: (x: 10mm, y: 12mm), footer: context [
  #set text(size: 8pt, dir: ltr)
  #align(center)[
    #str(counter(page).get().first()) از #str(counter(page).final().first())
  ]
])
#set text(font: "Vazirmatn", size: 9pt, dir: rtl, lang: "fa")

#align(center)[
  #text(size: 14pt, weight: "bold")[#d.header.organization_name]
  #linebreak()
  #text(size: 11pt)[#d.header.fiscal_year_caption]
  #linebreak()
  #text(size: 12pt, weight: "bold")[#d.header.report_title]
  #if d.header.period_caption != none [
    #linebreak()
    #text(size: 10pt)[#d.header.period_caption]
  ]
]
#v(4mm)

#let n = d.columns.len()
#table(
  columns: (1fr,) * n,
  align: right,
  stroke: 0.5pt,
  table.header(..d.columns),
  ..d.rows.flatten(),
  ..if d.totals != none { d.totals } else { () },
)

#if d.header.amount_in_words != none [
  #v(4mm)
  #text(size: 9pt)[مبلغ کل به حروف: #d.header.amount_in_words]
]

#if d.header.signature_labels.len() > 0 [
  #v(10mm)
  #grid(
    columns: (1fr,) * d.header.signature_labels.len(),
    ..d.header.signature_labels.map(s => align(center)[#s]),
  )
]
