-- Step 4.3 (docs/phase-4-treasury.md §4.3 / specs/06-treasury/06-04-endorsement-transfer-third-
-- party.md): a genuine third-party-transfer feature — the legacy reserves columns for this
-- (`S_Zssn`/`S_ZCR`/`S_ZName`, dropped entirely by step 4.1's migration) but never builds it (B14).
--
-- New terminal state, not reusing any of those dead legacy columns — the beneficiary is recorded on
-- the event row like every other transition's accounts, no new column needed on `received_cheques`
-- itself beyond a timestamp for symmetry with T4-T7's `deposited_at`/`cleared_at`/`bounced_at`/
-- `returned_at`.

-- PG12+: transactional, but the new value cannot be referenced by a literal in THIS same
-- transaction — nothing below does, so this is safe inside the migration's own transaction.
ALTER TYPE cheque_status ADD VALUE 'endorsed_to_third_party';

ALTER TABLE received_cheques ADD COLUMN endorsed_at date;

INSERT INTO journal_sources (id, code, label_fa, label_en, source_table) VALUES
  (27, 'cheque_endorsed', 'انتقال چک به شخص ثالث', 'Cheque endorsed to third party', 'cheques');
