// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::collections::BTreeMap;

use maplit::btreeset;
use mz_catalog::builtin::BuiltinTable;
use mz_catalog::durable::Transaction;
use mz_catalog::memory::objects::StateUpdate;
use mz_ore::collections::CollectionExt;
use mz_ore::now::NowFn;
use mz_persist_types::ShardId;
use mz_repr::{CatalogItemId, Timestamp};
use mz_sql::ast::display::AstDisplay;
use mz_sql_parser::ast::{Raw, Statement};
use mz_storage_client::controller::StorageTxn;
use semver::Version;
use tracing::info;
use uuid::Uuid;

// DO NOT add any more imports from `crate` outside of `crate::catalog`.
use crate::catalog::open::into_consolidatable_updates_startup;
use crate::catalog::state::LocalExpressionCache;
use crate::catalog::{BuiltinTableUpdate, CatalogState, ConnCatalog};

fn rewrite_ast_items<F>(tx: &mut Transaction<'_>, mut f: F) -> Result<(), anyhow::Error>
where
    F: for<'a> FnMut(
        &'a mut Transaction<'_>,
        CatalogItemId,
        &'a mut Statement<Raw>,
    ) -> Result<(), anyhow::Error>,
{
    let mut updated_items = BTreeMap::new();

    for mut item in tx.get_items() {
        let mut stmt = mz_sql::parse::parse(&item.create_sql)?.into_element().ast;
        f(tx, item.id, &mut stmt)?;

        item.create_sql = stmt.to_ast_string_stable();

        updated_items.insert(item.id, item);
    }
    tx.update_items(updated_items)?;
    Ok(())
}

fn rewrite_items<F>(
    tx: &mut Transaction<'_>,
    cat: &ConnCatalog<'_>,
    mut f: F,
) -> Result<(), anyhow::Error>
where
    F: for<'a> FnMut(
        &'a mut Transaction<'_>,
        &'a &ConnCatalog<'_>,
        CatalogItemId,
        &'a mut Statement<Raw>,
    ) -> Result<(), anyhow::Error>,
{
    let mut updated_items = BTreeMap::new();
    let items = tx.get_items();
    for mut item in items {
        let mut stmt = mz_sql::parse::parse(&item.create_sql)?.into_element().ast;

        f(tx, &cat, item.id, &mut stmt)?;

        item.create_sql = stmt.to_ast_string_stable();

        updated_items.insert(item.id, item);
    }
    tx.update_items(updated_items)?;
    Ok(())
}

/// Migrates all user items and loads them into `state`.
///
/// Returns the builtin updates corresponding to all user items.
pub(crate) async fn migrate(
    state: &mut CatalogState,
    tx: &mut Transaction<'_>,
    local_expr_cache: &mut LocalExpressionCache,
    item_updates: Vec<StateUpdate>,
    _now: NowFn,
    _boot_ts: Timestamp,
) -> Result<Vec<BuiltinTableUpdate<&'static BuiltinTable>>, anyhow::Error> {
    let catalog_version = tx.get_catalog_content_version();
    let catalog_version = match catalog_version {
        Some(v) => Version::parse(&v)?,
        None => Version::new(0, 0, 0),
    };

    info!(
        "migrating statements from catalog version {:?}",
        catalog_version
    );

    rewrite_ast_items(tx, |_tx, _id, _stmt| {
        // Add per-item AST migrations below.
        //
        // Each migration should be a function that takes `stmt` (the AST
        // representing the creation SQL for the item) as input. Any
        // mutations to `stmt` will be staged for commit to the catalog.
        //
        // Migration functions may also take `tx` as input to stage
        // arbitrary changes to the catalog.

        Ok(())
    })?;

    // Load items into catalog. We make sure to consolidate the old updates with the new updates to
    // avoid trying to apply unmigrated items.
    let commit_ts = tx.upper();
    let mut item_updates = into_consolidatable_updates_startup(item_updates, commit_ts);
    let op_item_updates = tx.get_and_commit_op_updates();
    let op_item_updates = into_consolidatable_updates_startup(op_item_updates, commit_ts);
    item_updates.extend(op_item_updates);
    differential_dataflow::consolidation::consolidate_updates(&mut item_updates);
    let item_updates = item_updates
        .into_iter()
        .map(|(kind, ts, diff)| StateUpdate {
            kind: kind.into(),
            ts,
            diff: diff.try_into().expect("valid diff"),
        })
        .collect();
    let mut ast_builtin_table_updates = state
        .apply_updates_for_bootstrap(item_updates, local_expr_cache)
        .await;

    info!("migrating from catalog version {:?}", catalog_version);

    let conn_cat = state.for_system_session();

    rewrite_items(tx, &conn_cat, |_tx, _conn_cat, _id, _stmt| {
        let _catalog_version = catalog_version.clone();
        // Add per-item, post-planning AST migrations below. Most
        // migrations should be in the above `rewrite_ast_items` block.
        //
        // Each migration should be a function that takes `item` (the AST
        // representing the creation SQL for the item) as input. Any
        // mutations to `item` will be staged for commit to the catalog.
        //
        // Be careful if you reference `conn_cat`. Doing so is *weird*,
        // as you'll be rewriting the catalog while looking at it. If
        // possible, make your migration independent of `conn_cat`, and only
        // consider a single item at a time.
        //
        // Migration functions may also take `tx` as input to stage
        // arbitrary changes to the catalog.
        Ok(())
    })?;

    // Add whole-catalog migrations below.
    //
    // Each migration should be a function that takes `tx` and `conn_cat` as
    // input and stages arbitrary transformations to the catalog on `tx`.

    let op_item_updates = tx.get_and_commit_op_updates();
    let item_builtin_table_updates = state
        .apply_updates_for_bootstrap(op_item_updates, local_expr_cache)
        .await;

    ast_builtin_table_updates.extend(item_builtin_table_updates);

    info!(
        "migration from catalog version {:?} complete",
        catalog_version
    );
    Ok(ast_builtin_table_updates)
}

// Add new migrations below their appropriate heading, and precede them with a
// short summary of the migration's purpose and optional additional commentary
// about safety or approach.
//
// The convention is to name the migration function using snake case:
// > <category>_<description>_<version>
//
// Please include the adapter team on any code reviews that add or edit
// migrations.

// Durable migrations

/// Migrations that run only on the durable catalog before any data is loaded into memory.
pub(crate) fn durable_migrate(
    tx: &mut Transaction,
    _organization_id: Uuid,
    _boot_ts: Timestamp,
) -> Result<(), anyhow::Error> {
    // Migrate the expression cache to a new shard. We're updating the keys to use the explicit
    // binary version instead of the deploy generation.
    const EXPR_CACHE_MIGRATION_KEY: &str = "expr_cache_migration";
    const EXPR_CACHE_MIGRATION_DONE: u64 = 1;
    if tx.get_config(EXPR_CACHE_MIGRATION_KEY.to_string()) != Some(EXPR_CACHE_MIGRATION_DONE) {
        if let Some(shard_id) = tx.get_expression_cache_shard() {
            tx.mark_shards_as_finalized(btreeset! {shard_id});
            tx.set_expression_cache_shard(ShardId::new())?;
        }
        tx.set_config(
            EXPR_CACHE_MIGRATION_KEY.to_string(),
            Some(EXPR_CACHE_MIGRATION_DONE),
        )?;
    }
    Ok(())
}

// Add new migrations below their appropriate heading, and precede them with a
// short summary of the migration's purpose and optional additional commentary
// about safety or approach.
//
// The convention is to name the migration function using snake case:
// > <category>_<description>_<version>
//
// Please include the adapter team on any code reviews that add or edit
// migrations.
