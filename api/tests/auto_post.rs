//! Automated version of step 2.5's manual test (docs/phase-2-accounting-
//! core.md §2.5): a balanced tuple set posts as a non-manual voucher; an
//! unbalanced set is rejected with nothing persisted; a bad account partway
//! through a multi-line call rolls back the whole thing.

use api::{auto_post::{post_generated_voucher, GeneratedLine, PostingError}, db};
use sqlx::PgPool;

struct Fixture {
    tenant_id: i64,
    fiscal_year_id: i64,
    cash_account_id: i64,
    revenue_account_id: i64,
    user_id: i64,
}

async fn seed(pool: &PgPool) -> Fixture {
    let tenant_id: i64 = sqlx::query_scalar("INSERT INTO tenants (slug, name) VALUES ('acme', 'Acme') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (tenant_id, username, password_hash) VALUES ($1, 'root', 'x') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let fiscal_year_id: i64 = sqlx::query_scalar(
        "INSERT INTO fiscal_years (tenant_id, year, start_date, end_date) \
         VALUES ($1, 1403, '2024-03-20', '2025-03-20') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let cash_account_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 1, 11, 'Cash') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let revenue_account_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, subsidiary_code, name) \
         VALUES ($1, 4, 41, 'Sales revenue') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .unwrap();
    Fixture { tenant_id, fiscal_year_id, cash_account_id, revenue_account_id, user_id }
}

#[sqlx::test(migrations = "./migrations")]
async fn balanced_tuples_post_as_a_non_manual_voucher(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let mut tx = db::begin(&pool, fx.tenant_id).await?;

    let lines = vec![
        GeneratedLine {
            account_id: fx.cash_account_id,
            debit: 1000,
            credit: 0,
            description: "Sale receipt".into(),
        },
        GeneratedLine {
            account_id: fx.revenue_account_id,
            debit: 0,
            credit: 1000,
            description: "Sale revenue".into(),
        },
    ];

    let voucher_id = post_generated_voucher(
        &mut tx,
        fx.tenant_id,
        fx.fiscal_year_id,
        chrono::NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(),
        "Auto-generated from invoice #1",
        1, // source_kind: inventory_invoice
        42,
        &lines,
        fx.user_id,
    )
    .await
    .expect("balanced tuples should post");
    tx.commit().await?;

    let (total_debit, total_credit, line_count): (i64, i64, i32) = sqlx::query_as(
        "SELECT total_debit, total_credit, line_count FROM vouchers WHERE id = $1",
    )
    .bind(voucher_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!((total_debit, total_credit, line_count), (1000, 1000, 2));

    let source_modules: Vec<i16> =
        sqlx::query_scalar("SELECT source_module FROM voucher_lines WHERE voucher_id = $1")
            .bind(voucher_id)
            .fetch_all(&pool)
            .await?;
    assert!(source_modules.iter().all(|&m| m == 1), "every line must be marked non-manual (source_kind 1)");

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn unbalanced_tuples_are_rejected_with_nothing_persisted(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let mut tx = db::begin(&pool, fx.tenant_id).await?;

    let lines = vec![
        GeneratedLine { account_id: fx.cash_account_id, debit: 1000, credit: 0, description: "x".into() },
        GeneratedLine { account_id: fx.revenue_account_id, debit: 0, credit: 500, description: "y".into() },
    ];

    let result = post_generated_voucher(
        &mut tx,
        fx.tenant_id,
        fx.fiscal_year_id,
        chrono::NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(),
        "Unbalanced",
        1,
        43,
        &lines,
        fx.user_id,
    )
    .await;
    assert!(matches!(result, Err(PostingError::Unbalanced)));
    tx.rollback().await?;

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM vouchers").fetch_one(&pool).await?;
    assert_eq!(count, 0, "no voucher row should exist after an unbalanced post");

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn a_bad_account_partway_through_leaves_nothing_written(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let mut tx = db::begin(&pool, fx.tenant_id).await?;

    let nonexistent_account_id = 999_999;
    let lines = vec![
        GeneratedLine { account_id: fx.cash_account_id, debit: 1000, credit: 0, description: "line1".into() },
        GeneratedLine { account_id: fx.revenue_account_id, debit: 0, credit: 500, description: "line2".into() },
        GeneratedLine { account_id: nonexistent_account_id, debit: 0, credit: 500, description: "line3 bad account".into() },
    ];

    let result = post_generated_voucher(
        &mut tx,
        fx.tenant_id,
        fx.fiscal_year_id,
        chrono::NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(),
        "Partial failure",
        1,
        44,
        &lines,
        fx.user_id,
    )
    .await;
    assert!(matches!(result, Err(PostingError::AccountNotFound(id)) if id == nonexistent_account_id));
    tx.rollback().await?;

    let voucher_count: i64 = sqlx::query_scalar("SELECT count(*) FROM vouchers").fetch_one(&pool).await?;
    let line_count: i64 = sqlx::query_scalar("SELECT count(*) FROM voucher_lines").fetch_one(&pool).await?;
    assert_eq!((voucher_count, line_count), (0, 0));

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn non_leaf_account_is_rejected(pool: PgPool) -> sqlx::Result<()> {
    let fx = seed(&pool).await;
    let non_leaf_id: i64 = sqlx::query_scalar(
        "INSERT INTO accounts (tenant_id, general_ledger_code, name, child_count) \
         VALUES ($1, 9, 'Non-leaf', 1) RETURNING id",
    )
    .bind(fx.tenant_id)
    .fetch_one(&pool)
    .await?;
    let mut tx = db::begin(&pool, fx.tenant_id).await?;

    let lines = vec![
        GeneratedLine { account_id: non_leaf_id, debit: 100, credit: 0, description: "x".into() },
        GeneratedLine { account_id: fx.cash_account_id, debit: 0, credit: 100, description: "y".into() },
    ];
    let result = post_generated_voucher(
        &mut tx,
        fx.tenant_id,
        fx.fiscal_year_id,
        chrono::NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(),
        "Non-leaf",
        1,
        45,
        &lines,
        fx.user_id,
    )
    .await;
    assert!(matches!(result, Err(PostingError::AccountNotLeaf(id)) if id == non_leaf_id));

    Ok(())
}
