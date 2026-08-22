_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 0. Reading notes, encoding, and dead code

### 0.1 Source encoding

The repository mixes two encodings. 22 `.pas` files are **Windows-1256 (Arabic/Persian codepage)**;
the remaining 227 `.pas`/`.dfm` files are **UTF-8** (most with a BOM). `.dfm` files additionally
encode non-ASCII string literals as Delphi `#NNNN` decimal escapes concatenated with quoted ASCII
runs, e.g. `Caption = #1587#1606#1583' 1'`.

Files confirmed Windows-1256 (relevant to this document): `NewSarfaslu.pas`, `ListSarfaslu.pas`,
`MakeRooznamehU.pas`. Everything else in scope is UTF-8.

**Migration consequence:** any tool that reads these sources must detect encoding per file. The same
applies to the SQL Server database itself, whose `varchar` columns are almost certainly stored under
an Arabic collation — the PostgreSQL migration must transcode to UTF-8.

### 0.2 Units that are dead code

Verified against `arzi.dpr` (the Delphi project file lists every compiled unit):

| Unit | In `arzi.dpr`? | Status |
|---|---|---|
| `FinalU.pas` | **No** | Dead. Superseded by `NewFinalu.pas`. Documented in §9.4 for reference only. |
| `KolSatateU.pas` | **No** | Dead. Contains a syntax error (`procesdure` at `KolSatateU.pas:35`) proving it is not compiled. Superseded by `KolStateU.pas`. |
| `S_KolU.pas` | **No** | Dead. Earlier version of `SNewu.pas`. |
| `NewSarfaslu.pas` | Yes | Compiled but **unreachable**: its only caller `TMain.Sarfasl_AddClick` begins with `exit;` (`Mainu.pas:576`), and the menu item is hard-disabled (`Mainu.pas:909`: `Sarfasl_Add.Enabled := False;`). |
| `ListSarfaslu.pas` | Yes | Compiled but **unreachable**: the call site is commented out (`Mainu.pas:570`). Replaced by `SNewu.pas`. |
| `ArticleMoeinu.pas` | Yes | Reachable only through `SanadMoeinu.pas` with `Kind=1`, which itself is now only entered for journal vouchers (`Kind=2`). Effectively legacy. |
| `MakeRooznamehU.pas` | Yes | Reachable from menu `SROOZ5` but superseded by `MoeinToRU.pas`. Both are documented (§8). |
| `SarfaslChap.pas` | Yes | A stub — `init` only calls `ShowModal`; its `Q1` has no SQL. Prints nothing. |

**Do not port dead units.** They are documented here only so that no behaviour is lost by accident,
and to record which of two competing implementations is the live one.

### 0.3 Server-side objects not present in this repository

The application calls 30 SQL Server stored procedures and at least two scalar functions. **None of
their definitions exist in this repository.** They must be extracted from the live database before
the rebuild can be considered complete. See §14 (Open questions).

Procedures/functions referenced by the accounting core:

| Object | Called from | Purpose (inferred) |
|---|---|---|
| `Sarfasl_ADD` | `SNewu.pas:628-640` | Insert an account; returns `_Error`, `_Desc` |
| `Sarfasl_Deep` | `ListSarfaslu.pas:172-178` | Delete an account with server-side guards; returns `M` (message) |
| `Sarfasl_view` | `ListSarfaslu.dfm:225` | Chart-of-accounts list for a company |
| `Sarfasl_Seek_SSN` | `Dmu.dfm:308` | Lookup account by id |
| `Sarfasl_Seek_Name` | `Dmu.dfm:329` | Lookup account by name |
| `Select_Kol` / `Select_moein` / `Select_Taf1` / `Select_Taf2` | `ArticleMoeinu.dfm`, `FGetCodeU.dfm` | Cascading account pickers |
| `Moein_All` | `BastanHesab.dfm:45` | All voucher lines for a company (used by the balance export) |
| `MoeinViewSanad` | `SanadMoeinu.dfm` | Voucher lines for one voucher |
| `MoeinAdd` | (data module) | Insert a voucher line |
| `MoeinTotalSanad` | (data module) | Voucher totals |
| `Moein_ChapSanad` | `SanadViewU.dfm:599` | Voucher print dataset |
| `Moein_View_Daftar` | `Dmu.dfm:85` | Subsidiary ledger |
| `KolState` | `KolStateU.dfm` | General-ledger control list |
| `Taraz4Setooni` / `Taraz_6Sotooni` | `Dmu.dfm:34`, report units | 4- and 6-column trial balances |
| `Active_Set` | `SNewu.pas:303`, `SNewu.pas:346` | Rebuilds denormalised `Sarfasl` columns after a code/name change |
| `XNew` | `Dmu.pas:1234` | Returns `CurrentDate` (Jalali) for a company |
| `dbo.Make_R(coid, ko, mo, ta1, ta2)` | `Dmu.pas:278` (commented), `RoyatJU.pas:367` | Builds the right-to-left code string `M_R` |
| `dbo.Make_L(coid, ko, mo, ta1, ta2)` | `Dmu.pas:274` (commented) | Builds the left-to-right code string `M_L` |

---

_Next: [03-01-a-account-hierarchy-model](03-01-a-account-hierarchy-model.md)_
