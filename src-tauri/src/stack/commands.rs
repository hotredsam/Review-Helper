//! Stack commands: read the catalog + pre-made stacks, list/override selections,
//! and apply a pre-made stack.

use std::collections::HashMap;

use tauri::State;

use super::{
    apply_premade, catalog, list_selections, premade, set_selection, CatalogOption, PremadeStack,
    Selection,
};
use crate::db::Db;

#[tauri::command]
pub fn stack_catalog() -> HashMap<String, Vec<CatalogOption>> {
    catalog().clone()
}

#[tauri::command]
pub fn stack_premade() -> Vec<PremadeStack> {
    premade().to_vec()
}

#[tauri::command]
pub fn stack_list(db: State<'_, Db>, project_id: i64) -> Result<Vec<Selection>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_selections(&conn, project_id)
}

#[tauri::command]
pub fn stack_set(db: State<'_, Db>, project_id: i64, pane: String, choice: String) -> Result<(), String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    set_selection(&mut conn, project_id, &pane, &choice)
}

#[tauri::command]
pub fn stack_apply_premade(db: State<'_, Db>, project_id: i64, name: String) -> Result<(), String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    apply_premade(&mut conn, project_id, &name)
}
