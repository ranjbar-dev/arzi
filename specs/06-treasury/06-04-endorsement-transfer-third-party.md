_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 4. Endorsement / transfer to a third party

**Endorsement is not implemented. The `Z*` columns are dead schema.**

### 4.1 What exists

Three columns on `DCheck` (`Dmu.dfm:982-991`):

```
object DCheckS_Zssn: TIntegerField   FieldName = 'S_Zssn'
object DCheckS_ZCR:  TStringField    FieldName = 'S_ZCR'    Size = 50
object DCheckS_ZName: TStringField   FieldName = 'S_ZName'  Size = 100
```

They are mirrored by three persistent fields on the cheque list's `Q1`
(`CheckListDU.pas:96-98`, `CheckListDU.dfm:783-793`), which is a `Select *` so they arrive
automatically.

### 4.2 What proves it is unimplemented

A repository-wide search for `S_Zssn`, `S_ZCR`, `S_ZName` and the bare token `Zssn` returns exactly
these hits and nothing else:

- the three `TField` declarations in `Dmu.dfm` and the matching `Dmu.pas:75-77`;
- the three `TField` declarations in `CheckListDU.dfm` / `CheckListDU.pas`;
- **three commented-out schema-migration calls** in `Dmu.pas:249-251`:

```pascal
//     CreateFieldInTable('Dcheck' , 'S_Zssn' , 'int' );
//     CreateFieldInTable('Dcheck' , 'S_ZCR' , 'Varchar(50)' );
//     CreateFieldInTable('Dcheck' , 'S_ZName' , 'Varchar(100)' );
```

There is **no** assignment to any of them, **no** read of any of them, **no** endorsement screen,
**no** menu entry, **no** button on `CheckListDU` (the complete `OnClick` inventory of that form is
listed in §2.1), **no** permission key between the used range 2102-2109, **no** reserved `M_Id`, and
**no** `DCheck2` state code for it. The three columns exist in the ADO field list because the
developer ran the migration once (the calls are commented out *after* having been executed) and then
abandoned the feature.

Nothing in `CheckListDU`'s grid displays them either — the grid columns are enumerated in
`CheckListDU.dfm:60-150` and none of the `Z*` fields is among them.

### 4.3 The intended design, inferred

The naming is consistent across the schema: `Bed*` = debit-side account triplet, `Bes*` =
credit-side account triplet, `Z*` = a third triplet in the same shape (id, code string, name). The
glossary reads `Z` as *Zi-nafa* (ذی‌نفع, "beneficiary"). Combined with the unused state code 3 and the
unwired `State3` speed button (`Tag = 3`, `CheckListDU.dfm:317-327`), the abandoned design was
almost certainly:

> state 1 (in hand) → **endorse to beneficiary Z** → a terminal state, with a voucher debiting the
> beneficiary's account and crediting the notes-receivable-on-hand account — structurally identical
> to T7 (return to issuer) but pointing at a third party instead of the original payer.

That is a guess from schema shape, not from code. **Nothing in the repository confirms it.**

### 4.4 How endorsement is actually done today

The operator's only route is `CheckEsterdadU` (T7, §2.3): return the cheque to whoever gave it,
which reverses the receipt posting, then settle with the third party separately. The endorsement
itself is invisible to the system — the physical cheque leaves the company with no record of where
it went.

### 4.5 Consequence for the rebuild

The three columns should **not** be ported as-is. Either (a) drop them and implement endorsement
properly as a first-class transition with its own event type and voucher, or (b) drop them and
leave endorsement out of scope. Carrying dead nullable columns into PostgreSQL preserves nothing,
because no historical row has ever had a value in them. Raised for decision in §12 and §13.


---

[← 3. Received versus issued cheques](06-03-received-versus-issued-cheques.md) | [index](00-index.md) | [5. Due-date logic →](06-05-due-date-logic.md)
