_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 6.5 String comparison is the *only* date arithmetic — and its consequences

Every range filter, every sort, every "is this before that" test is lexicographic:

```
DKolU.pas:285-286     Qs.SQL.Add('  And M_Date >='+ QuotedStr(D1.Farsi_Date) );
                      Qs.SQL.Add('  And M_Date <='+ QuotedStr(D2.Farsi_Date) );
DMoein.pas:651-652    (identical)
Taraz4Setooni_U.pas:103   _W := 'Where M_Date<='+QuotedStr(D1.Farsi_Date)+' and M_kind=1 and M_COID='
DaftarT_U.pas:149-150 ' and M_date>=' + QuotedStr(D1.Farsi_Date) + ' and M_date<='+ QuotedStr(D2.Farsi_Date)
Anbar_Amalkard.pas:177,201,227   ' and AFD_Date>= '+QuotedStr(D1.Farsi_Date)+' and AFD_Date <= '+QuotedStr(D2.Farsi_Date)
MoeinZipU.pas:258,590,600,649,672
Report7U.pas:403,437 ; RoyatJU.pas:308 ; AnbarReportU.pas:210
```

and ordering:

```
DKolU.pas:276,287 / DMoein.pas:640,653 / TMoein.pas:239,249   ORDER BY M_Date, M_Sanad
DaftarT_U.pas:159                                             Order By M_Date, M_Sanad
Report7U.pas:440                                              order by M_Date, M_Sanad
MoeinZipU.pas:340,603                                         Order By M_Date, X, K, M, T1, T2
AnbarListU.pas:546                                            Order By AF_Date Desc, AF_Factor Desc
Anbar_Amalkard.pas:179,204                                    Order by AFD_Date, AFD_Factor
RooznamehViewU.pas:139                                        Order BY DM_Date
CheckListDU.pas:329 / FishListD.pas:231                       Order By S_Dates
```

**This works — and only works — while every value is exactly `YYYY/MM/DD`, zero-padded.**
Zero-padded fixed-width `YYYY/MM/DD` is lexicographically order-isomorphic to the real calendar
order, which is why the design survived at all.

It breaks in three identified ways:

1. **Mixed widths.** `Farsi_Date` on the third-party `TEditDate` control yields the **8-character**
   form in at least one place — `AnbarReportKharidU.pas:96,98` does
   `D1.Text := '13'+_D1.Farsi_Date;`, which is only meaningful if `Farsi_Date` is `YY/MM/DD`.
   Meanwhile `Anbar_MandehU.pas:115-118` compares that same property directly against the
   10-character `Dm.To_Date` / `Dm.From_Date`:
   ```pascal
   if D1.Farsi_Date >  Dm.To_Date Then D1.Farsi_Date := Dm.To_Date;
   if D1.Farsi_Date <  Dm.From_Date Then D1.Farsi_Date := Dm.From_Date;
   ```
   `'99/01/01' > '1400/01/01'` is **true** as strings (`'9' > '1'`), so a clamp intended to pull a
   date inside the fiscal year can push it outside. Any row written with an 8-char date sorts after
   *every* 10-char date. **The rebuild must scan production data for `LENGTH(date_column) <> 10`.**
2. **No leap validation** (§6.4) admits `1400/12/30`, which sorts correctly but denotes a
   non-existent day; converting it to a real `date` during migration will fail.
3. **`SQL_VARIANT`/collation.** The comparison runs server-side under the column's collation, not
   ordinal. With the Persian/Arabic code pages in play this is fine for ASCII digits and `/`, but
   it is an implicit dependency worth pinning.

Also note `Get2D.pas:67` uses the same trick to enforce "from ≤ to":
`(N1.Text < N2.Text)` — string comparison again.

### 6.6 `DateS` is not always a date

`S_DateS` on `DCheck` is declared `TStringField` with **`Size = 50`** (`Dmu.dfm:947-950`), while
`S_Date` alongside it is `Size = 10` (`Dmu.dfm:943-946`). A 50-character column is not a date. The
`DCheck` variants of `DateS` therefore carry free text (or a date plus an annotation) and must be
inspected in live data before migration. Contrast `CheckListDU.pas:329` / `FishListD.pas:231`,
which `Order By S_Dates` — a **third** spelling, and a different column. See §2 and §12.

### 6.7 Date-entry UI

- `TEditDate` (unit `Tools`, third-party, **source not in this repository**) — properties used:
  `.Text` (full string), `.Farsi_Date` (see §6.5.1 caveat), `.Farsi_year`
  (`MakeNewU.pas:75,77`, where the new fiscal year is created by `Farsi_year := Farsi_year + 1`).
- `GetD.pas` — modal single-date prompt, seeded from `DM.Current_Date` when blank
  (`GetD.pas:38-39`), OK enabled only when `Dm.isValidDate` passes (`GetD.pas:49`).
- `Get2D.pas` — modal date-range prompt, OK enabled only when both dates validate *and*
  `N1.Text < N2.Text` (`Get2D.pas:66-68`).
- Report date ranges are persisted to the ini file as strings and read back
  (`AnbarReportU.pas:132-133, 176-177`) — see §8.

### 6.8 Proposed PostgreSQL model

**Store the real date; derive the Jalali representation.**

| Concern | Proposal |
|---|---|
| Business date column | `date NOT NULL` (Gregorian, `DATE` type). Name: `<thing>_date` e.g. `voucher_date`, `due_date`, `invoice_date`. |
| Jalali presentation | **Not stored.** Computed at the edge. The Rust backend converts with a vetted crate (e.g. `ptime` / `jalali` bindings to the 33-year-cycle algorithm) and serialises **both** `date` (ISO) and `dateJalali` (`YYYY/MM/DD`) in API responses. React renders `dateJalali`; the API accepts either on input. |
| Ordering / ranges | native `date` comparison and `BETWEEN`. All the lexicographic hacks in §6.5 disappear. |
| Fiscal-year bounds | `fiscal_years.start_date date NOT NULL`, `fiscal_years.end_date date NOT NULL`, with `CHECK (end_date > start_date)`; the `isValidDate` rule becomes a DB `CHECK` or a service-layer guard plus an exclusion constraint on overlapping years. |
| Audit stamps | `created_at timestamptz NOT NULL DEFAULT now()`, `updated_at timestamptz` — replacing `GetDate()`. Always `timestamptz`, never naked `timestamp`. |
| "Today" | resolved on the **server** in the Rust layer (preserving the legacy property that the DB clock is authoritative, `Dmu.pas:1232-1239`), not in the browser. Store the timezone `Asia/Tehran` in config. |

**Migration rules (must be applied, not assumed):**

1. Reject/quarantine any legacy value where `LEN(col) <> 10`, `col NOT LIKE '[0-9][0-9][0-9][0-9]/[0-9][0-9]/[0-9][0-9]'`, month `> 12`, or day `> 31`.
2. Reject `MM/30` where `MM = 12` and the Jalali year is not leap under the 33-year rule; these are
   the artefacts of the missing leap check (`Dmu.pas:897`). Decide per-row with the business owner
   whether they mean `12/29` of the same year or `01/01` of the next.
3. Convert with a **single, correct** Jalali→Gregorian implementation, not a port of either
   algorithm in §6.3. Verify round-trip on the full distinct-date set before cutting over.
4. Keep the original string in a `legacy_date_jalali text` shadow column for the first release so
   discrepancies are auditable, then drop it.

Open items for §12: the body of `XNew` (which conversion it actually performs), the true width of
`TEditDate.Farsi_Date`, and the meaning of the 50-character `DCheck.S_DateS`.


---

[← 02-06-a-date-handling-storage-and-algorithms.md](02-06-a-date-handling-storage-and-algorithms.md) | [02-07-money-and-amount-handling.md →](02-07-money-and-amount-handling.md)
