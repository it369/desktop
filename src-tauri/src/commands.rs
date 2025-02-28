use tauri::api::path::{document_dir, download_dir, home_dir};
use tauri::command;
use tauri::Runtime;
use tauri::State;

use std::collections::HashMap;
use std::io::ErrorKind;

use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::Path;
use uuid::Uuid;

use crate::menu::{self, MenuActionRequest};
use crate::utils::SystemInfoWithPreference;
use crate::{preference, utils};
use onekeepass_core::db_service as kp_service;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UpdateType {
  GroupUpdate,
  EntryUpdate,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UpdatePayload {
  pub update_type: UpdateType,
}

pub type Result<T> = std::result::Result<T, String>;

#[command]
pub(crate) async fn load_kdbx(
  db_file_name: &str,
  password: &str,
  key_file_name: Option<&str>,
  app_state: State<'_, utils::AppState>,
) -> Result<kp_service::KdbxLoaded> {
  // key_file_name.as_deref() converts Option<String> to Option<&str> - https://stackoverflow.com/questions/31233938/converting-from-optionstring-to-optionstr
  let r = kp_service::load_kdbx(db_file_name, password, key_file_name.as_deref());

  if let Err(kp_service::error::Error::DbFileIoError(m, ioe)) = &r {
    // Remove from the recent list only if the file opening failed because of the file is not found in the passed file path
    if let ("Database file opening failed", ErrorKind::NotFound) = (m.as_str(), ioe.kind()) {
      app_state
        .preference
        .lock()
        .unwrap()
        .remove_recent_file(db_file_name);
      return Ok(r?);
    }
  }

  // Appends this file name to the most recently opened file list
  app_state
    .preference
    .lock()
    .unwrap()
    .add_recent_file(db_file_name);
  Ok(r?)
}

/// UI layer redirects any backend menu actions
#[tauri::command]
pub(crate) async fn menu_action_requested<R: Runtime>(
  app: tauri::AppHandle<R>,
  request: MenuActionRequest,
) -> Result<()> {
  menu::menu_action_requested(request, app);
  Ok(())
}

#[command]
pub(crate) async fn is_path_exists(in_path: String) -> bool {
  Path::new(&in_path).exists()
}

#[tauri::command]
pub(crate) async fn standard_paths<R: Runtime>(
  app: tauri::AppHandle<R>,
) -> HashMap<String, Option<String>> {
  let mut m: HashMap<String, Option<String>> = HashMap::new();

  if let Some(p) = document_dir() {
    m.insert(
      "document-dir".into(),
      p.as_path().to_str().map(|s| s.into()),
    );
  }

  if let Some(p) = home_dir() {
    m.insert("home-dir".into(), p.as_path().to_str().map(|s| s.into()));
  }

  if let Some(p) = download_dir() {
    m.insert(
      "download-dir".into(),
      p.as_path().to_str().map(|s| s.into()),
    );
  }

  match utils::app_resources_dir(app) {
    Ok(r) => {
      info!("Resource dir is {}", r);
      m.insert("resources-dir".into(), Some(r));
    }
    Err(x) => {
      error!("Resource dir is not found with error {:?}", x)
    }
  };
  m
}

#[tauri::command]
pub(crate) async fn read_app_preference(
  app_state: State<'_, utils::AppState>,
) -> Result<preference::Preference> {
  let g = app_state.preference.lock().unwrap();
  Ok(g.clone())
}

#[tauri::command]
pub(crate) async fn system_info_with_preference(
  app_state: State<'_, utils::AppState>,
) -> Result<SystemInfoWithPreference> {
  Ok(SystemInfoWithPreference::init(app_state.inner()))
}

#[tauri::command]
pub(crate) async fn get_db_settings(db_key: &str) -> Result<kp_service::DbSettings> {
  Ok(kp_service::get_db_settings(db_key)?)
}

#[tauri::command]
pub(crate) async fn set_db_settings(
  db_key: &str,
  db_settings: kp_service::DbSettings,
) -> Result<()> {
  Ok(kp_service::set_db_settings(db_key, db_settings)?)
}

#[tauri::command]
pub(crate) async fn create_kdbx(
  new_db: kp_service::NewDatabase,
  app_state: State<'_, utils::AppState>,
) -> Result<kp_service::KdbxLoaded> {
  let r = kp_service::create_kdbx(new_db)?;
  // Appends this file name to the most recently opned file list
  app_state
    .preference
    .lock()
    .unwrap()
    .add_recent_file(&r.db_key);
  Ok(r)
}

#[command]
pub(crate) async fn move_group_to_recycle_bin(db_key: &str, group_uuid: Uuid) -> Result<()> {
  Ok(kp_service::move_group_to_recycle_bin(db_key, group_uuid)?)
}

#[command]
pub(crate) async fn move_group(db_key: &str, group_uuid: Uuid, new_parent_id: Uuid) -> Result<()> {
  Ok(kp_service::move_group(db_key, group_uuid, new_parent_id)?)
}

#[command]
pub(crate) async fn move_entry_to_recycle_bin(db_key: &str, entry_uuid: Uuid) -> Result<()> {
  Ok(kp_service::move_entry_to_recycle_bin(db_key, entry_uuid)?)
}

#[command]
pub(crate) async fn move_entry(db_key: &str, entry_uuid: Uuid, new_parent_id: Uuid) -> Result<()> {
  Ok(kp_service::move_entry(db_key, entry_uuid, new_parent_id)?)
}

#[command]
pub(crate) async fn remove_group_permanently(db_key: &str, group_uuid: Uuid) -> Result<()> {
  Ok(kp_service::remove_group_permanently(db_key, group_uuid)?)
}

#[command]
pub(crate) async fn remove_entry_permanently(db_key: &str, entry_uuid: Uuid) -> Result<()> {
  Ok(kp_service::remove_entry_permanently(db_key, entry_uuid)?)
}

#[command]
pub(crate) async fn empty_trash(db_key: &str) -> Result<()> {
  Ok(kp_service::empty_trash(db_key)?)
}

#[command]
pub(crate) async fn kdbx_context_statuses(db_key: &str) -> Result<kp_service::KdbxContextStatus> {
  Ok(kp_service::kdbx_context_statuses(db_key)?)
}

#[command]
pub(crate) async fn get_entry_form_data_by_id(
  db_key: &str,
  entry_uuid: Uuid,
) -> Result<kp_service::EntryFormData> {
  Ok(kp_service::get_entry_form_data_by_id(&db_key, &entry_uuid)?)
}

#[command]
pub(crate) async fn history_entry_by_index(
  db_key: &str,
  entry_uuid: Uuid,
  index: i32,
) -> Result<kp_service::EntryFormData> {
  Ok(kp_service::history_entry_by_index(
    &db_key,
    &entry_uuid,
    index,
  )?)
}

#[command]
pub(crate) async fn delete_history_entry_by_index(
  db_key: &str,
  entry_uuid: Uuid,
  index: i32,
) -> Result<()> {
  Ok(kp_service::delete_history_entry_by_index(
    &db_key,
    &entry_uuid,
    index,
  )?)
}

#[command]
pub(crate) async fn delete_history_entries(db_key: &str, entry_uuid: Uuid) -> Result<()> {
  Ok(kp_service::delete_history_entries(&db_key, &entry_uuid)?)
}

#[command]
pub(crate) async fn groups_summary_data(db_key: String) -> Result<kp_service::GroupTree> {
  Ok(kp_service::groups_summary_data(&db_key)?)
}

#[tauri::command]
pub(crate) async fn entry_summary_data(
  db_key: String,
  entry_category: kp_service::EntryCategory,
) -> Result<Vec<kp_service::EntrySummary>> {
  Ok(kp_service::entry_summary_data(&db_key, entry_category)?)
}

#[tauri::command]
pub async fn history_entries_summary(
  db_key: &str,
  entry_uuid: Uuid,
) -> Result<Vec<kp_service::EntrySummary>> {
  Ok(kp_service::history_entries_summary(&db_key, &entry_uuid)?)
}

#[tauri::command]
pub(crate) async fn new_entry_form_data(
  db_key: &str,
  entry_type_uuid: Uuid,
  parent_group_uuid: Option<Uuid>,
) -> Result<kp_service::EntryFormData> {
  Ok(kp_service::new_entry_form_data_by_id(
    db_key,
    &entry_type_uuid,
    parent_group_uuid.as_ref().as_deref(),
  )?)
}

#[tauri::command]
pub(crate) async fn entry_type_headers(db_key: &str) -> Result<kp_service::EntryTypeHeaders> {
  Ok(kp_service::entry_type_headers(db_key)?)
}

#[tauri::command]
pub(crate) async fn insert_or_update_custom_entry_type(
  db_key: &str,
  entry_type_form_data: kp_service::EntryTypeFormData,
) -> Result<Uuid> {
  Ok(kp_service::insert_or_update_custom_entry_type(
    db_key,
    &entry_type_form_data,
  )?)
}

#[tauri::command]
pub(crate) async fn delete_custom_entry_type(
  db_key: &str,
  entry_type_uuid: Uuid,
) -> Result<kp_service::EntryTypeHeader> {
  Ok(kp_service::delete_custom_entry_type_by_id(
    db_key,
    &entry_type_uuid,
  )?)
}

#[tauri::command]
pub(crate) async fn get_group_by_id(db_key: String, group_uuid: Uuid) -> Result<kp_service::Group> {
  Ok(kp_service::get_group_by_id(&db_key, &group_uuid)?)
}

#[tauri::command]
pub(crate) async fn update_group(
  db_key: String,
  group: kp_service::Group,
  window: tauri::Window,
) -> Result<()> {
  kp_service::update_group(&db_key, group)?;
  // Leaving it here as example to send an event from a command
  // let _r = window.emit(
  //   "group_update",
  //   UpdatePayload {
  //     update_type: UpdateType::GroupUpdate,
  //   },
  // );
  Ok(())
}

#[tauri::command]
pub(crate) async fn update_entry_from_form_data(
  db_key: &str,
  form_data: kp_service::EntryFormData,
) -> Result<()> {
  Ok(kp_service::update_entry_from_form_data(db_key, form_data)?)
}


#[tauri::command]
pub(crate) async fn insert_entry_from_form_data(
  db_key: &str,
  form_data: kp_service::EntryFormData,
) -> Result<()> {
  Ok(kp_service::insert_entry_from_form_data(db_key, form_data)?)
}

#[tauri::command]
pub(crate) async fn new_blank_group(mark_as_category: bool) -> kp_service::Group {
  kp_service::new_blank_group(mark_as_category)
}

#[tauri::command]
pub(crate) async fn insert_group(db_key: String, group: kp_service::Group) -> Result<()> {
  Ok(kp_service::insert_group(&db_key, group)?)
}

#[tauri::command]
pub(crate) async fn get_categories_to_show(
  db_key: String,
) -> Result<kp_service::EntryCategoryInfo> {
  Ok(kp_service::categories_to_show(&db_key)?)
}

#[tauri::command]
pub(crate) async fn mark_group_as_category(
  db_key: String,
  group_id: String,
  window: tauri::Window,
) -> Result<()> {
  kp_service::mark_group_as_category(&db_key, &group_id)?;
  //As the group data is modified, the "group_update" event is emitted and appropriate listener
  //in the UI reacts accordingly
  // let _r = window.emit(
  //   "group_update",
  //   UpdatePayload {
  //     update_type: UpdateType::GroupUpdate,
  //   },
  // );
  Ok(())
}

#[command]
pub(crate) async fn upload_entry_attachment(
  db_key: &str,
  file_name: &str,
) -> Result<kp_service::AttachmentUploadInfo> {
  Ok(kp_service::upload_entry_attachment(db_key, file_name)?)
}

#[command]
pub(crate) async fn save_as_kdbx(
  db_key: &str,
  db_file_name: &str,
  app_state: State<'_, utils::AppState>,
) -> Result<kp_service::KdbxLoaded> {
  let r = kp_service::save_as_kdbx(db_key, db_file_name)?;
  // Appends this file name to the most recently opned file list
  app_state
    .preference
    .lock()
    .unwrap()
    .add_recent_file(db_file_name);
  Ok(r)
}

#[command]
pub(crate) async fn save_kdbx(
  db_key: &str,
  app_state: State<'_, utils::AppState>,
) -> Result<kp_service::KdbxSaved> {
  // db_key is the full database file name and backup file name is derived from that
  let backup_file_name = app_state.get_backup_file(db_key);
  Ok(kp_service::save_kdbx_with_backup(
    db_key,
    backup_file_name.as_deref(),
  )?)
}

#[tauri::command]
pub(crate) async fn save_all_modified_dbs(
  db_keys: Vec<String>,
  app_state: State<'_, utils::AppState>,
) -> Result<Vec<kp_service::SaveAllResponse>> {
  // Need to prepare back file paths for all db_keys 
  let dbs_with_backups: Vec<(String, Option<String>)> = db_keys
    .iter()
    .map(|s| (s.clone(), app_state.get_backup_file(s)))
    .collect();

  Ok(kp_service::save_all_modified_dbs_with_backups(
    dbs_with_backups,
  )?)
}

#[command]
pub(crate) async fn close_kdbx(db_key: &str) -> Result<()> {
  Ok(kp_service::close_kdbx(db_key)?)
}

#[command]
pub(crate) async fn unlock_kdbx(
  db_key: &str,
  password: &str,
  key_file_name: Option<&str>,
) -> Result<kp_service::KdbxLoaded> {
  Ok(kp_service::unlock_kdbx(db_key, password, key_file_name)?)
}

#[command]
pub(crate) async fn collect_entry_group_tags(db_key: &str) -> Result<kp_service::AllTags> {
  Ok(kp_service::collect_entry_group_tags(db_key)?)
}

#[command]
pub(crate) async fn search_term(db_key: &str, term: &str) -> Result<kp_service::EntrySearchResult> {
  Ok(kp_service::search_term(db_key, term)?)
}

#[command]
pub(crate) async fn analyzed_password(
  password_options: kp_service::PasswordGenerationOptions,
) -> Result<kp_service::AnalyzedPassword> {
  Ok(kp_service::analyzed_password(password_options)?)
}

#[command]
pub(crate) async fn score_password(password: &str) -> Result<kp_service::PasswordScore> {
  Ok(kp_service::score_password(password))
}

#[command]
pub(crate) async fn export_main_content_as_xml(db_key: &str, xml_file_name: &str) -> Result<()> {
  // This will just export the main content
  Ok(kp_service::export_main_content_as_xml(
    db_key,
    xml_file_name,
  )?)
}

#[command]
pub(crate) async fn export_as_xml(db_key: &str, xml_file_name: &str) -> Result<()> {
  // This will refresh struct before xml export
  Ok(kp_service::export_as_xml(db_key, xml_file_name)?)
}

#[tauri::command]
pub async fn load_custom_svg_icons<R: Runtime>(
  app: tauri::AppHandle<R>,
) -> Result<HashMap<String, String>> {
  Ok(utils::load_custom_svg_icons(app))
}

// TODO: Remove this or need to clean up if required
// Leaving it here as example for the future use if any
#[tauri::command]
pub async fn svg_file<R: Runtime>(app: tauri::AppHandle<R>, name: &str) -> Result<String> {
  //let ad = utils::app_resources_dir(app.package_info());

  use tauri::{
    api::path::{app_config_dir, home_dir, resolve_path, resource_dir, runtime_dir, BaseDirectory},
    Env,
  };

  println!(
    "Resources dir is {:?}",
    resource_dir(app.package_info(), &Env::default())
  );
  println!("Home dir is {:?}", home_dir());
  println!("Runtime dir {:?}", runtime_dir());
  println!("App Config dir {:?}", app_config_dir(&app.config().clone()));

  let path = resolve_path(
    &app.config(),
    app.package_info(),
    &Env::default(),
    "../resources/public/icons/custom-svg",
    Some(BaseDirectory::Resource),
  )
  .unwrap();
  println!("resolved path  is {:?}", path);
  let svg_path = resource_dir(app.package_info(), &Env::default())
    .unwrap()
    .join("_up_/resources/public/icons/custom-svg")
    .join(name);
  println!("svg_path  is {:?}", svg_path);
  let s = read_to_string(svg_path).unwrap();
  //Ok("Done".into())
  Ok(s)
}
