_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 10. Aggregation / consolidation — `TajmiU.pas`

Menu: `_Report9` → `'مشاهده دفتر تجمیعی'` ("view the consolidated ledger") — `Mainu.pas:679-682`.

**Purpose:** for a Moein account that has Tafsil-1 children, show the turnover and balance of each
Tafsil-1 child **rolled up across all its Tafsil-2 descendants**. A two-pane drill-down report, not a
data-modifying operation.

Panel instruction (`TajmiU.pas:77`):
`'برای مشاهده دفتر معین تجمیعی کد مورد نظر را انتخاب کنید'`
("To view the consolidated subsidiary ledger, select the desired code").

**Left pane** — the selectable Moein accounts (`TajmiU.pas:88-93`, matching `TajmiU.dfm:240-243`):

```sql
 Select S.*
 From Sarfasl As S
 Where S_Mo>0 And S_Ta1=0 And S_Child>0
 Order By S_Ko, S_Mo
```

i.e. **Moein-level nodes that have at least one Tafsil-1 child**. A Moein with no children is a
posting account and has nothing to consolidate.

**Right pane** — refreshed on every left-pane cursor move (`TajmiU.pas:69-72`, `:96-121`):

```sql
 IF OBJECT_ID('tempdb..#R') IS NOT NULL Drop TABLE #R

 Select M_Ko, M_Mo, M_Ta1
  , M_R=(Select M_R from sarfasl Where M_Ko=S_Ko and M_Mo=S_Mo and S_Ta1=M_Ta1 and S_Ta2=0)
  , M_L=(Select M_L from sarfasl Where M_Ko=S_Ko and M_Mo=S_Mo and S_Ta1=M_Ta1 and S_Ta2=0)
  , FullName=(Select Fullname from sarfasl Where M_Ko=S_Ko and M_Mo=S_Mo and S_Ta1=M_Ta1 and S_Ta2=0)
  , Sum(M_Bed) As Bed , Sum(M_Bes) As Bes
  , Sum(M_Bed-M_Bes) As BedR
  , Sum(M_Bes-M_Bed) As BesR
 into #R From moein
 Where M_Ko=<selected Ko>
 and M_Mo=<selected Mo>
 Group By M_Ko, M_Mo, M_Ta1

 Update #R Set BedR=0 Where BedR < 0
 Update #R Set BesR=0 Where BesR < 0

 Select * from #R
```

**What it merges and why:** `Group By M_Ko, M_Mo, M_Ta1` **drops `M_Ta2`**. Every posting at
`Ta1 = k, Ta2 = anything` collapses into one row for `Ta1 = k`. The Tafsil-2 level is a
sub-classification (in this installation typically a per-person or per-lot breakdown under a Tafsil-1
category); the consolidated ledger shows the category totals without the detail.

The correlated sub-selects resolve the display strings from the **Tafsil-1 header row** (`S_Ta2 = 0`),
which is the node being summarised.

Output columns:

| Column | Meaning |
|---|---|
| `M_Ko`, `M_Mo`, `M_Ta1` | The consolidated account |
| `M_R`, `M_L` | Display code, RTL / LTR |
| `FullName` | `/`-joined name path |
| `Bed`, `Bes` | **Gross** turnover (debit / credit) |
| `BedR`, `BesR` | **Net** balance, clamped to ≥ 0 (§9.1) |
| `BedS`, `BesS`, `BedRS`, `BesRS` | The same four, formatted with thousands separators by `Q2CalcFields` (`TajmiU.pas:128-136`) using `DM.inttostr3` |

`Dm.inttostr3` (`Dmu.pas:859-867`) inserts `,` at 3, 7 and 11 characters from the right — i.e. groups
of **3-4-4**, not uniform 3-3-3:

```pascal
   S:= inttostr( N );
   if Length( S ) > 3 Then S := Copy( S , 1 , Length(S)-3)  + ',' + Copy( S , Length(S)-2,  3 );
   if Length( S ) > 7 Then S := Copy( S , 1 , Length(S)-7)  + ',' + Copy( S , Length(S)-6,  7 );
   if Length( S ) >11 Then S := Copy( S , 1 , Length(S)-11) + ',' + Copy( S , Length(S)-10, 11 );
   if N=0 then Result:= DefaluZero Else Result:= S;
```

Deliberate — the Iranian convention for grouping large rial amounts. Reproduce it, or replace with a
locale formatter after checking with the users. Note the second parameter `DefaluZero` (default
`'0'`): callers pass `''` to render zero as an empty cell (`TajmiU.pas:131-134`).

Deletion is blocked on both grids (`TajmiU.pas:123-126`: `Q1BeforeDelete` → `Abort`).

`_Kind` is set to 1 for "Moein consolidation" (`TajmiU.pas:45`, `:80`) — a second mode
(`2 = tafzil`) was planned and never implemented.

---

_Prev: [03-09-c-period-close-and-year-end](03-09-c-period-close-and-year-end.md) | Next: [03-11-index-of-all-sql-in-the-accounting-core](03-11-index-of-all-sql-in-the-accounting-core.md)_
