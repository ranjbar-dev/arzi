_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 4.5 Design-time dataset SQL in `Dmu.dfm`

The 27 `TADOStoredProc` declarations are covered in **§3.1** and not repeated. The `TADOQuery`
datasets follow. Bare `TADOTable` components (`Base`, `Moein`, `Sarfasl`, `Sahamdar`, `Tanzim`,
`Password`, `DCheck`, `DFish`, `AnbarConfig`, `AnbarJens`, `AnbarFactor`, `AnbarFactorD`,
`Kind_Table`, `TCheck`) carry **no SQL at all** — ADO generates `SELECT * FROM <table>` and the
client-side cursor does the filtering, which is §9.2's optimistic-concurrency mechanism.

#### 4.5.1 `Anbar_Jens_Phi1` — item stock and moving-average price

```sql
Declare @C int      Set @C = :Code
Declare @CID int    Set @CID= :CoID
Declare @F int      Set @F =:Factor

Declare @Noin Real
Set @Noin   = (Select sum(AFD_num) from Anbar_FactorD where AFD_Code=@C and AFD_Type=1 and AFD_Coid=@CID and AFD_Factor<> @F )
Declare @NoOut Real
Set @NoOut  = (Select sum(AFD_num) from Anbar_FactorD where AFD_Code=@C and AFD_Type=2 and AFD_Coid=@CID and AFD_Factor<> @F )
Declare @NoBin Real
Set @NoBin  = (Select sum(AFD_num) from Anbar_FactorD where AFD_Code=@C and AFD_Type=3 and AFD_Coid=@CID and AFD_Factor<> @F)
Declare @NoBOut Real
Set @NoBOut = (Select sum(AFD_num) from Anbar_FactorD where AFD_Code=@C and AFD_Type=4 and AFD_Coid=@CID and AFD_Factor<> @F)

Declare @Mabin bigint
Set @Mabin   = ( Select sum( AFD_Num * AFD_Phi ) From Anbar_FactorD Where AFD_Code=@C and AFD_Type=1 and AFD_Coid=@CID and AFD_Factor<> @F)
Declare @MabOut bigint
Set @MabOut  = ( Select sum( AFD_Num * AFD_Phi ) From Anbar_FactorD Where AFD_Code=@C and AFD_Type=2 and AFD_Coid=@CID and AFD_Factor<> @F)
Declare @MabBin bigint
Set @MabBin  = ( Select sum( AFD_Num * AFD_Phi ) From Anbar_FactorD Where AFD_Code=@C and AFD_Type=3 and AFD_Coid=@CID and AFD_Factor<> @F)
Declare @MabBOut bigint
Set @MabBOut = ( Select sum( AFD_Num * AFD_Phi ) From Anbar_FactorD Where AFD_Code=@C and AFD_Type=4 and AFD_Coid=@CID and AFD_Factor<> @F)

Declare @Phiin int
Set @Phiin = 0
if @Noin >0  Set @phiin = Cast( @Mabin / @Noin as int )

Select  Anbar_Jens.*
       ,@Noin as Noin , @Mabin as Mabin , @phiin as phiin , @NOOut As NoOut, @MabOut As MabOut
       ,@NoBIn As NoBin ,@MabbIn As MabBIn , @NoBOut As NoBOut ,@MabBOut as MabBOut
       , (@Noin - @NoOut - @NoBin + @NoBOut) As Remi
From Anbar_Jens
where AJ_code=@C
```
`Dmu.dfm:657-717`. Consumer: `AnbarFactorAddU.pas:114-133`.

**Intent.** For one item, in one fiscal year, **excluding the invoice currently being edited**
(`AFD_Factor <> @F`): total quantity and value in and out by document type, the **moving-average
purchase price** `@Phiin`, and the on-hand quantity `Remi`.

**Findings — this is the most consequential ad-hoc statement in the data module.**

- **`AF_Type` semantics are settled here**, filling a gap §12.9 could not: `1` = inbound, `2` =
  outbound, `3` = inbound *return* (`برگشت`), `4` = outbound return. The balance formula
  `Noin − NoOut − NoBin + NoBOut` confirms that type 3 **reduces** stock and type 4 **increases**
  it — i.e. 3 is a *purchase return* (goods going back out) and 4 a *sales return* (goods coming
  back in), despite 3 sharing the "in" naming. ⚠ **Add this to §11.6's `document_type` comment.**
- **Quantities are declared `Real`** — a 4-byte float — while the column is `decimal(_,3)`
  (§7.3). Summing thousands of movements through `Real` loses precision. This is the third place
  §3.4 item 2's "quantity as float" problem appears.
- **`@Phiin` is an integer division cast**: `Cast(@Mabin / @Noin as int)`. `@Mabin` is `bigint` and
  `@Noin` is `Real`, so SQL Server promotes to `float`, divides, then **truncates toward zero**.
  That is the moving-average cost, and it silently loses up to one rial per unit — matching the
  invoice path's truncation rule (§7.4, §13.16).
- **`AFD_Factor <> @F` is the "exclude myself" trick** that lets the screen recompute availability
  while an invoice is open. It is also why the query cannot be replaced by a simple aggregate view.
- On-hand quantity is **never stored** — it is a full re-scan of `Anbar_FactorD` on every keystroke
  (`05-inventory.md` §5.1). With no index on `(AFD_Code, AFD_Coid, AFD_Type)` this is eight
  full-table aggregates per item lookup. §11.6's `inventory_invoice_lines_stock_idx` addresses it.

**Rebuild.** `inventory::stock_on_hand(item_id, fiscal_year_id, exclude_invoice_id)` — one query
with `FILTER (WHERE …)` aggregates, `numeric` throughout, and the truncation applied once,
explicitly, by the named rounding function of §7.7.

#### 4.5.2 `Sarfasl_ChildCount` — how many children does this node have?

```sql
Declare @Kol int    Set @Kol=:Kol
Declare @Moein int  Set @Moein=:moein
Declare @Taf1 int   Set @Taf1=:Taf1
Declare @Taf2 int   Set @Taf2=:Taf2

Declare @R int Set @R=0

if @Kol=0   Set @R = (Select Count(*) From Sarfasl Where S_Mo=0)
if @Kol>0   Set @R = (Select Count(*) From Sarfasl Where S_Ko=@Kol and S_Mo>0 and S_Ta1=0)
if @moein>0 Set @R = (Select Count(*) From Sarfasl Where S_Ko=@Kol and S_Mo=@moein and S_Ta1>0 and S_Ta2=0)
if @taf1>0  Set @R = (Select Count(*) From Sarfasl Where S_Ko=@Kol and S_Mo=@moein and S_Ta1=@Taf1 and S_Ta2>0)
if @taf2>0  Set @R = 0

Select @R As ChildCount
```
`Dmu.dfm:838-870`

**Intent.** The single-node form of §4.1.2 — count the direct children of the node identified by
however many segments were supplied, returning `0` for a level-4 node (always a leaf).

**Findings.**

- The four `if`s are **cascading, not exclusive**: each later condition overwrites `@R`. It happens
  to be correct because the segments fill left to right, which is exactly the invariant §11.3's
  `accounts_segment_hierarchy` `CHECK` formalises. A row violating that invariant makes this query
  return the wrong count.
- Note the **parameters are declared `ftWideString`** in the `.dfm` (`Size = 3`, values `'413'`,
  `'111'`) and assigned into `int` locals — ADO performs the conversion. A non-numeric value would
  raise at the server, not the client.
- This is the live, per-node counterpart to the stale stored `S_Child` column: **the application
  does not trust `S_Child` enough to read it here.** Strong support for §13.6 (derive it).

#### 4.5.3 `Anbar_Tasfieh` — invoice settlement (cheques + slips against one invoice)

```sql
Declare @Sal int Set @Sal = :Sal
Declare @Sanad int
Declare @Factor int Set @Factor =:Factor
Declare @Jari int
Set @Jari  = ( Select min(AF_Customer) From Anbar_Factor Where AF_Coid=@Sal and AF_Factor=@Factor )
set @SAnad = ( Select min(AF_Sanad)    From Anbar_Factor Where AF_Coid=@Sal and AF_Factor=@Factor )

Select 22 AS _ID , S_StateName as _SN, S_SSN As _SSN , S_FishNo As _Link  , S_Mab As _Mab ,
         S_Desc As _Desc , '' As _DSar
--   From DFish Where S_Coid=@Sal And S_Sanad=@Sanad and S_BesSSN=@Jari
   From DFish Where S_Coid=@Sal And S_LinkPRG=1 and S_LinkSSN=@Factor

union
Select 21 AS _ID , S_StateName as _SN, S_SSN As _SSN , S_CheckNo As _Link  , S_Mab As _Mab ,
         S_Desc As _Desc , S_DateS as _DSar
--   From DCheck Where S_Coid=@Sal And S_Sanad=@Sanad and S_BesSSN=@Jari
   From DCheck Where S_Coid=@Sal And S_LinkPRG=1 and S_LinkSSN=@Factor
```
`Dmu.dfm:1034-1070`, with persistent result fields at `Dmu.dfm:1073-1105`.

**Intent.** `تصفیه` = settlement. List every payment instrument — deposit slips and received
cheques — that was created *from* a given inventory invoice, so the screen can show how much of the
invoice has been settled.

**Findings.**

- **The commented-out predicates record an abandoned design.** The original matched on
  `S_Sanad = @Sanad AND S_BesSSN = @Jari` — same voucher, same counterparty. It was replaced by the
  explicit `S_LinkPRG = 1 AND S_LinkSSN = @Factor` link. `@Jari` and `@Sanad` are still computed and
  now **entirely unused** — dead work on every execution.
- **`S_LinkSSN` holds the invoice *number*, not the invoice's `AF_SSN`.** The join is
  `S_LinkSSN = @Factor` where `@Factor` is the invoice number — so the polymorphic pointer for
  treasury documents is keyed on a *business* number scoped by year, not a surrogate key.
  §11.5 models `source_id bigint`; the migration must resolve `(fiscal_year_id, invoice_number)` →
  `inventory_invoices.id`, **not** copy `S_LinkSSN` verbatim. ⚠ Not previously recorded.
- The literals `22` and `21` in the `_ID` column are `M_ID` source-module codes (§2.7) — but note
  `22` is used here for a **deposit slip**, whereas §2.7's table maps `25` to deposit slips and `22`
  to a bounced cheque. **The two numbering schemes disagree**; `_ID` here is a local
  result-set discriminator, not an `M_ID`. Do not conflate them (a trap for the porting team).
- `UNION` (not `UNION ALL`) de-duplicates across two heterogeneous sources — harmless but wasteful.
- `_DSar` is `S_DateS` for cheques and `''` for slips, confirming that `S_DateS` is the cheque's due
  date and reinforcing §12.2.

**Rebuild.** `GET /api/v1/inventory-invoices/{id}/settlements`, a `UNION ALL` over `cheques` and
`deposit_slips` filtered by `source_module`/`source_id`, with a typed discriminator.

#### 4.5.4 `SahamdarConfig` — mark which account templates a party already has

```sql
Declare @Card int Set @Card=:Card
Declare @Kind int Set @kind=:Kind

Update SahamdarConfig Set SC_Tik=0
Update SahamdarConfig Set SC_Tik= 1 Where Exists(
    Select * From Sarfasl Where Sarfasl.S_Ko=SahamdarConfig.SC_K and Sarfasl.S_Mo=SahamdarConfig.SC_M
                            and Sarfasl.S_Ta1=@Card and Sarfasl.S_Ta2=0)

Select * From SahamdarConfig Where SC_Kind=@Kind or SC_Tik=1
```
`Dmu.dfm:8625-8636`

**Intent.** For a given party card number, tick the Kol/Moein template rows for which that party
**already has** a chart-of-accounts node, then return the templates applicable to the party's kind
plus the ticked ones.

**Findings — a serious one.**

- **A `SELECT` dataset performs two unfiltered `UPDATE`s on a shared table as a side effect.**
  `Update SahamdarConfig Set SC_Tik=0` touches **every row**, for every user, every time any user
  opens the party screen. `SC_Tik` is a **global scratch flag masquerading as a column**. Two
  concurrent users produce interleaved, wrong results — this is a concrete instance of §9.2's
  "no conflict handling" and belongs in §9.9's failure-mode list.
- It is also the clearest source-level statement of the party↔account rule §2.6 describes:
  a party's account is `(SC_K, SC_M, S_Card, 0)` — **`Sahamdar.S_Card` occupying the `S_Ta1`
  segment**, with `S_Ta2 = 0`.
- `SahamdarConfig` columns exposed here: `SC_K`, `SC_M`, `SC_T`, `SC_Kind`, `SC_Tik`, `SC_Rem`.
  See §4.4.2 — this table is **absent from §2's master list and from §11**.

**Rebuild.** A pure read: `SELECT t.*, EXISTS(SELECT 1 FROM accounts a WHERE …) AS has_account FROM
account_templates t WHERE t.party_type = $2`. No writes, no scratch column.

#### 4.5.5 `Jari_Rem` — per-party balance across account templates

```sql
Declare @Jari int Set @Jari=:Jari
Declare @Sal int  Set @Sal=:Sal

   if OBJECT_ID('tempdb..#R') is not null Drop Table #R

   Select  SC_K, SC_M, SC_T as SC_T1, 0 As SC_T2 ,
           Cast(0 as Bigint) As Bed, Cast(0 as Bigint) As Bes, Cast(0 as Bigint) As Rem
   into #R
   From SahamdarConfig
   Where SC_Rem = 1

   Update #R Set SC_T2=@Jari Where SC_T2=0 and SC_T1>0
   Update #R Set SC_T1=@Jari Where SC_T1=0

   Update #R Set Bes = isnull( ( Select Sum(M_Bes) From Moein Where M_Ko=#R.SC_K and M_Mo=#R.SC_M
                                 and M_Ta1=#R.SC_T1 and M_Ta2=#R.SC_T2 and M_Coid=@Sal) , 0)
   Update #R Set Bed = isNull( ( Select Sum(M_Bed) From Moein Where … ) , 0)
   …
```
`Dmu.dfm:8658-8690+` (truncated above at the point where the pattern repeats)

**Intent.** `جاری` = current account. For one party in one fiscal year, produce a debit/credit/net
line per account template flagged `SC_Rem = 1`.

**Findings.**

- **A `#temp` table built and mutated inside a `TADOQuery`.** It is dropped defensively at the top
  because ADO's connection pooling may hand back a session where `#R` still exists — an admission
  that connection reuse is unmanaged (§9.5).
- The two `UPDATE #R SET SC_T…=@Jari` statements place the party's card number into **either the
  `S_Ta1` or the `S_Ta2` segment** depending on whether the template already specifies a Tafsil1.
  So the party↔account encoding of §2.6 is **not uniform**: a party can be a level-3 *or* a level-4
  node depending on the template. ⚠ **This complicates §13.7's migration** — `accounts.party_id`
  must be back-filled from *both* `analytic1_code` and `analytic2_code`, per template. Not
  previously recorded.
- Balances are computed from the **denormalised `M_Ko`/`M_Mo`/`M_Ta1`/`M_Ta2` columns on `Moein`**,
  not from `M_Code`. §11.4 keeps those only as `legacy_*` migration columns — this query is the
  reason they cannot simply be dropped on day one.
- This is the third independent implementation of "sum the ledger for an account" (alongside
  `Moein_All` and `EnteghalU`'s `#R` query, §12.3 item 9). **All three must be diffed.**

#### 4.5.6 The remaining `Dmu.dfm` datasets

| Dataset | SQL | Location | Intent / finding |
|---|---|---|---|
| `AnbarCala_SeekName` | `Declare @Name Varchar(20)` … `Set @Name='%'+Ltrim(RTrim(:Name))+'%'` … `Select * From Anbar_Jens Where ((PATINDEX(@Name, AJ_Name) > 0) or (Len(@Name)=2)) Order by AJ_ID, AJ_Code` | `Dmu.dfm:614-625` | Item search by substring. **`Len(@Name)=2` means "the two wildcards only" — i.e. an empty search returns everything.** Leading-wildcard `PATINDEX` is a full scan; §11.6 proposes a trigram index. The local is `Varchar(20)` while `AJ_Name` is longer — a search term over 18 characters is silently truncated. |
| `QCheck` | `Select * from TCheck` | `Dmu.dfm:900-903` | Unfiltered. `BeforeDelete = QCheckBeforeDelete`. No column of `TCheck` is referenced anywhere (§11.5). |
| `QDCheck` | `Select * from DCheck` | `Dmu.dfm:912-915` | Unfiltered — **no `S_COID`**, so it spans every fiscal year. Used only for its `BeforeDelete` guard. |
| `QDFish` | `Select * from DFish` | `Dmu.dfm:1010-1013` | Same, for deposit slips. Same cross-year exposure. |
| `KharidPeste_List` | `Select * From NewRamz` | `Dmu.dfm:1118-1121` | On `ADO_RPPCSOLUTION` — the **external** pistachio-receipt catalog (§1.5, row E6). Unfiltered `SELECT *` across a foreign system's table. Do not port; integrate (§11.7). |
| `E_K` | `Select * From Password` | `Dmu.dfm:21-22` | §4.2.2. |
| `Base_Q`, `UpdateQ` | the `CO_DESC` query | `Dmu.dfm:367-372`, `:879-884` | §4.2.3. |
| `Q1`, `Q2`, `QS`, `QS1`, `Q1ADO` | *(no design-time SQL)* | `Dmu.dfm:351`, `:629`, `:1166`, `:591`, `:1137` | Empty shells whose `.SQL` is built at runtime by the `Dmu.pas` methods of §4.1–§4.4. `Q1ADO` and `QS1` are the ones bound to the auxiliary connections (§1.5). |

**Note the connection string in plain text.** `ADO_RPPCSOLUTION.ConnectionString`
(`Dmu.dfm:1126-1131`) contains `User ID=sa` and `Data Source=Pesteh` **uncommented and
unobfuscated** in a checked-in file — unlike the main connection, which is obfuscated in the ini
(§1.3). Whatever protection the `53269` scheme was meant to provide, this defeats it. Add to
`08-platform-and-security.md`'s findings and to §13.17's rationale.

---


---

[← 02-04-b-adhoc-sql-accounting-and-lookups.md](02-04-b-adhoc-sql-accounting-and-lookups.md) | [02-04-d-adhoc-sql-misc-and-summary.md →](02-04-d-adhoc-sql-misc-and-summary.md)
