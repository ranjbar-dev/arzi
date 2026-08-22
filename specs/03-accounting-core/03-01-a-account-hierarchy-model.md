_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 1. The account hierarchy model

### 1.1 The `Sarfasl` table

`Sarfasl` (proposed: **`accounts`**) is a single flat table holding **all four levels** of the chart
of accounts. Declared as `TADOTable` with `TableName = 'Sarfasl'` at `Dmu.dfm:376-378`.

Complete column list, recovered from the FastReport dataset field-alias map at
`FactorPrint3U.dfm:3299-3331` (this is the only place in the repository where the full schema is
enumerated):

| Column | Type (inferred) | Meaning | Proposed name |
|---|---|---|---|
| `S_SSN` | `int identity` PK | Surrogate key. Referenced everywhere as the stable account handle. | `id` |
| `S_Ko` | `int` | Level-1 code (Kol / general ledger) | `level1_code` |
| `S_Mo` | `int` | Level-2 code (Moein / subsidiary). `0` when the row *is* a Kol node. | `level2_code` |
| `S_Ta1` | `int` | Level-3 code (Tafsil 1 / analytic). `0` above that level. | `level3_code` |
| `S_Ta2` | `int` | Level-4 code (Tafsil 2 / sub-analytic). `0` above that level. | `level4_code` |
| `S_Name` | `varchar(50)` | Account name at this node only (not the full path) | `name` |
| `S_Child` | `int` | **Denormalised count of direct children.** `0` ⇒ leaf ⇒ postable. | `child_count` (see §1.6) |
| `S_Lock` | `int` (0/1) | Node lock. Blocks posting for non-admins, inherited down the tree. | `is_locked` |
| `S_Active` | `int` (0/1) | Written as `'1'` on create (`NewSarfaslu.pas:213`). **Never read anywhere.** | `is_active` (see §14) |
| `S_Kind` | `int` | **Present in the schema, never written or read by any live code.** Candidate account-nature column that was never implemented. See §14. | — |
| `S_Card` | `int` | FK to `Sahamdar.S_Card` — links an analytic account to a person/shareholder record (`ListSarfaslu.pas:233`, `ListSarfaslu.pas:315`) | `party_id` |
| `S_Bed` | `bigint` | Denormalised debit total. Written as `'0'` on create (`NewSarfaslu.pas:209`); **never maintained afterwards**. | drop |
| `S_Bes` | `bigint` | Denormalised credit total. Same. (`NewSarfaslu.pas:210`) | drop |
| `S_Remi` | `bigint` | Denormalised balance. Same. (`NewSarfaslu.pas:211`, `Mainu.pas:728`) | drop |
| `S_Count` | `int` | Denormalised line count. Same. (`NewSarfaslu.pas:212`, `Mainu.pas:725`) | drop |
| `M_R` | `varchar` | Denormalised **right-to-left** display code, built by `dbo.Make_R` | derived, drop |
| `M_L` | `varchar` | Denormalised **left-to-right** display code, built by `dbo.Make_L` | derived, drop |
| `FullName` | `varchar(200)` | Denormalised `/`-joined name path (see the disabled builder at `Dmu.pas:284-295`) | derived, drop |
| `LineName` | `varchar` | Denormalised "last-level code + name" label, read at `SanadEditU.pas:373` | derived, drop |
| `NeedUpdate` | `int` | Dirty flag set before calling `Active_Set` to rebuild the denormalised columns (`SNewu.pas:277`, `SNewu.pas:338`) | drop |
| `S_Address` | `varchar(100)` | Party address | `address` |
| `S_Tel` | `varchar(20)` | Phone | `phone` |
| `S_Fax` | `varchar(20)` | Fax | `fax` |
| `S_Melli` | `varchar` | Iranian national ID (کد ملی) | `national_id` |
| `S_Sabt` | `varchar` | Company registration number (شماره ثبت) | `registration_number` |
| `S_Egh` | `varchar` | Economic code (کد اقتصادی) | `economic_code` |
| `S_Post` | `varchar` | Postal code (کد پستی) | `postal_code` |
| `S_IS_Check` | `int` (0/1) | Flags an account as a cheque account. **Write path is commented out** (`Sarfasl_TakmilU.pas:75-76`). Superseded by `base_config`. | drop |
| `S_IS_Fish` | `int` (0/1) | Deposit-slip account. Same — commented out (`Sarfasl_TakmilU.pas:77-78`). | drop |
| `S_IS_APArdakhti` | `int` (0/1) | Notes-payable account. Same (`Sarfasl_TakmilU.pas:79-80`). | drop |
| `S_IS_ADaryafti` | `int` (0/1) | Notes-receivable account. Same (`Sarfasl_TakmilU.pas:81-82`). | drop |
| `S_COID` | `int` | Company/fiscal-year scope. Referenced at `Sarfasl_ListU.pas:44` and `MakeNewU.pas:144`, but **every live query ignores it** — see §1.7. | see §1.7 |

### 1.2 The four levels

| Level | Column | Persian | Glossary term | Role |
|---|---|---|---|---|
| 1 | `S_Ko` | کل | Kol | General ledger head |
| 2 | `S_Mo` | معین | Moein | Subsidiary account |
| 3 | `S_Ta1` | تفصیل ۱ | Tafsil 1 | Analytic account |
| 4 | `S_Ta2` | تفصیل ۲ | Tafsil 2 | Sub-analytic account |

There is no fifth level. `SNewu.pas:66` states it explicitly:

```pascal
State : integer ;   // 1 Kol  2 Moein  3 Taf1   4 Taf2
```

**The representation is a materialised path, not an adjacency list.** Every row carries all four
code components; the level of a row is determined by which trailing components are zero:

- Kol node: `S_Ko > 0, S_Mo = 0, S_Ta1 = 0, S_Ta2 = 0`
- Moein node: `S_Ko > 0, S_Mo > 0, S_Ta1 = 0, S_Ta2 = 0`
- Tafsil-1 node: `S_Ko > 0, S_Mo > 0, S_Ta1 > 0, S_Ta2 = 0`
- Tafsil-2 node: `S_Ko > 0, S_Mo > 0, S_Ta1 > 0, S_Ta2 > 0`

These predicates are used verbatim in the level queries at `SNewu.pas:488`, `SNewu.pas:503`,
`SNewu.pas:521`, `SNewu.pas:539`, and in the pickers at `SelectSarfasl.pas:94`,
`SelectSarfasl.pas:111`, `SelectSarfasl.pas:128`, `SelectSarfasl.pas:145`.

The natural key is therefore the tuple `(S_Ko, S_Mo, S_Ta1, S_Ta2)`, and `S_SSN` is a surrogate over
it. Both are used as lookup keys throughout; `Dm.Sarfasl_Seek` (`Dmu.pas:1152-1157`) locates by the
tuple, `Dm.Moein_SeekSSN` and `TarafU.Set_SSN` (`TarafU.pas:232-252`) by the surrogate.

### 1.3 Code digit widths

Digit widths are **configured per fiscal year**, stored on the `Base` table:

| `Base` column | Applies to | Read at |
|---|---|---|
| `No_Ko` | `S_Ko` | `NewSarfaslu.pas:61`, `ArticleMoeinu.pas:93`, `Dmu.pas:1200` |
| `No_Mo` | `S_Mo` | `NewSarfaslu.pas:65`, `ArticleMoeinu.pas:94`, `Dmu.pas:1206` |
| `No_Ta1` | `S_Ta1` | `NewSarfaslu.pas:68`, `ArticleMoeinu.pas:95`, `Dmu.pas:1213` |
| `No_Ta2` | `S_Ta2` | `NewSarfaslu.pas:71`, `ArticleMoeinu.pas:96`, `Dmu.pas:1220` |

They are edited on the settings screen (`TanzimU.pas:131-134`) and are **display/input widths only** —
the stored values are plain integers, never zero-padded in the database.

Observed defaults, from the hard-coded fallbacks in the next-code generator (`SNewu.pas:553`,
`SNewu.pas:563`) and the zero-padding rules in the level queries (`SNewu.pas:504`, `SNewu.pas:522`):
Kol = 3 digits (first code `111`), Moein = 3 digits (first code `111`, padded to length 3),
Tafsil-1 = 4 digits (padded to length 4, first code `1`), Tafsil-2 = unconstrained (first code `1`).

**Important:** the code is *not* a concatenation. A child's code is **not derived** from its parent's
digits — each level has an independent integer that is only unique *within* its parent. The full path
is the tuple. This is different from most Iranian ERPs (which concatenate) and must be preserved.

### 1.4 Code string formats

Four mutually incompatible textual renderings of the same code exist in the codebase. All four must
be reproduced (the UI shows them in different places), but the rebuild should compute all of them
from the tuple rather than storing them.

**(a) Left-to-right, unpadded, dash-joined** — `Ko-Mo-Ta1-Ta2`, omitting trailing zero levels.

```pascal
// SanadEditU.pas:357-365
function TSanadEditF.Make_CStr(K, M, T1, T2: integer): String;
var S:String;
begin
   S:= inttostr(K);
   if M>0  then S:= S+'-'+ inttostr(M);
   if T1>0 then S:= S+'-'+ inttostr(T1);
   if T2>0 then S:= S+'-'+ inttostr(T2);
   Result := S;
end;
```

Same format at `TarafU.pas:104-113` (`Get_FullCode`) and `FGetCodeU.pas:85-87` (`Make_L`). This is
the format the user *types* — `Dm.Split_Code` (`Dmu.pas:510-543`) and `TarafU.Set_FullCode`
(`TarafU.pas:189-230`) parse exactly this.

**(b) Right-to-left, zero-padded, dash-joined** — `Ta2-Ta1-Mo-Ko`, each component padded to its
configured width. Built by `Dm.Sarfasl_SSN_CODEName` (`Dmu.pas:1180-1229`):

```pascal
// Dmu.pas:1199-1223 (abridged)
// Kol
    L  := Base.FieldByName('NO_Ko').AsInteger;
    S1 := '00000000'+Sarfasl.FieldByName('S_Ko').asstring ; S1:= Copy( S1, Length(S1)-L+1 , L );
    S :=  S1;
// Moein
    if Sarfasl.FieldByName('S_Mo').AsInteger > 0 Then
    Begin
       L  := Base.FieldByName('NO_Mo').AsInteger;
       S1 := '00000000'+Sarfasl.FieldByName('S_Mo').asstring ; S1:= Copy( S1, Length(S1)-L+1 , L );
       S := S1 + '-'+ S;
    End;
// ... same for S_Ta1, S_Ta2 ...
// Add Name
    S := S +' ' + Trim( Sarfasl.FieldByName('S_Name').asstring );
```

Note the prefixing (`S1 + '-' + S`) — the deepest level ends up leftmost. This is the visual RTL
rendering for a Persian reader.

**(c) `M_R` / `M_L`** — pre-computed columns produced server-side by `dbo.Make_R` / `dbo.Make_L`.
The grid lets the user pick which of the two to display (`ListSarfaslu.pas:204-219`,
`SanadMoeinu.pas:412-427`), persisting the choice in the INI file under key `M_RL`.

**(d) `FullName`** — `/`-joined **names** (not codes) down the path. The (now-disabled) builder is at
`Dmu.pas:284-295`; the runtime equivalent is `TarafU.Get_FullName` (`TarafU.pas:126-135`):

```pascal
    if EKo.Tag>0  then S:=SKo.Text;
    if EMo.Tag>0  then S:=S+'/'+SMO.Text;
    if ETa1.Tag>0 then S:=S+'/'+STa1.Text;
    if ETa2.Tag>0 then S:=S+'/'+STa2.Text;
```

A per-line variant using `#13#10` instead of `/` is at `TarafU.pas:115-124` (`Get_FullCodeName`).

### 1.5 Uniqueness rules

The only uniqueness rule enforced anywhere is on the full tuple:

- `NewSarfaslu.pas:196-201` — locate on `S_ko;S_mo;S_ta1;S_ta2`; if found, reject with
  `'کد وارد شده تکراري است'` ("The entered code is duplicate").
- `SNewu.pas:266-270` — when renumbering a node, checks the *sibling set currently loaded in the
  grid* for the new code and rejects with `' کد داده شده تکراری است و تغییر غیر قابل اجرا است '`
  ("The given code is duplicate and the change cannot be executed").

Account **names are not unique** and are not checked for uniqueness anywhere.

**Gap:** the duplicate check in `SNewu.pas:266` uses `Q1.Locate` against the in-memory grid dataset,
which contains only the siblings at the current level under the current parent. That is correct by
accident for levels 2–4, and correct for level 1 too since the Kol grid holds all Kol nodes. But it
is a client-side check with no database constraint behind it. The rebuild **must** add a unique
index on `(company_id, level1_code, level2_code, level3_code, level4_code)`.

---

_Prev: [03-00-reading-notes-encoding-dead-code](03-00-reading-notes-encoding-dead-code.md) | Next: [03-01-b-account-hierarchy-model](03-01-b-account-hierarchy-model.md)_
