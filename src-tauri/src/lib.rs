use billhub_core::{ImportFile, ImportOptions, LedgerStore, preview};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, FilePath};

#[derive(Serialize)]
struct ApiError {
    code: String,
    message: String,
}

impl ApiError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            code: "IMPORT_FAILED".into(),
            message: message.into(),
        }
    }
}

impl<T> From<T> for ApiError
where
    T: std::fmt::Display,
{
    fn from(value: T) -> Self {
        Self::new(value.to_string())
    }
}

type ApiResult<T> = Result<T, ApiError>;

struct AppState {
    #[allow(dead_code)]
    database_path: PathBuf,
    store: Mutex<LedgerStore>,
}

impl AppState {
    fn lock(&self) -> ApiResult<std::sync::MutexGuard<'_, LedgerStore>> {
        self.store.lock().map_err(|_| ApiError::new("数据库忙"))
    }
}

#[tauri::command]
fn api_preview(
    file_path: String,
    state: State<AppState>,
) -> ApiResult<billhub_core::ImportPreview> {
    let file = ImportFile::from_path(&file_path)?;
    Ok(preview(&*state.lock()?, &file)?)
}

#[tauri::command]
fn api_import(
    file_path: String,
    replace: bool,
    task_day: String,
    state: State<AppState>,
) -> ApiResult<billhub_core::BatchRecord> {
    let file = ImportFile::from_path(&file_path)?;
    let store = state.lock()?;
    let parsed = billhub_core::detect_import_file(&file)?.parse(&file)?;
    let result = store.import_with_task(
        &file,
        parsed,
        ImportOptions {
            replace,
            source_path: Some(file.file_name.clone()),
        },
        Some(&task_day),
    )?;
    Ok(result)
}

#[tauri::command]
fn api_create_manual_entry(occurred_at:i64, amount_cents:i64, cash_flow:String, category:String, description:String, task_day:String, state:State<AppState>) -> ApiResult<billhub_core::LedgerEventRecord> {
    Ok(state.lock()?.create_manual_entry(occurred_at, amount_cents, &cash_flow, &category, &description, &task_day)?)
}

#[tauri::command]
fn api_gamification(day:String, state:State<AppState>) -> ApiResult<billhub_core::GamificationSnapshot> { Ok(state.lock()?.gamification(&day)?) }

#[tauri::command]
fn api_complete_daily_task(task_id:String, day:String, state:State<AppState>) -> ApiResult<billhub_core::GamificationSnapshot> {
    if task_id != "review_transactions" { return Err(ApiError::new("只允许完成每日回顾任务")); }
    Ok(state.lock()?.complete_review_task(&day)?)
}

#[tauri::command]
fn api_batches(state: State<AppState>) -> ApiResult<Vec<billhub_core::BatchRecord>> {
    Ok(state.lock()?.batches()?)
}

#[tauri::command]
fn api_events(
    page: usize,
    page_size: usize,
    provider: Option<String>,
    cash_flow: Option<String>,
    lifecycle: Option<String>,
    period: Option<String>,
    state: State<AppState>,
) -> ApiResult<billhub_core::EventPage> {
    Ok(state
        .lock()?
        .event_page_filtered(page, page_size, provider, cash_flow, lifecycle, period)?)
}

#[tauri::command]
fn api_update_event(
    event_id: String,
    event_kind: String,
    lifecycle: String,
    state: State<AppState>,
) -> ApiResult<bool> {
    Ok(state
        .lock()?
        .update_event_classification(&event_id, &event_kind, &lifecycle)?)
}

#[tauri::command]
fn api_update_events(
    event_ids: Vec<String>,
    event_kind: Option<String>,
    lifecycle: Option<String>,
    state: State<AppState>,
) -> ApiResult<usize> {
    Ok(state.lock()?.update_events_classification(
        &event_ids,
        event_kind.as_deref(),
        lifecycle.as_deref(),
    )?)
}

#[tauri::command]
fn api_summary(
    include_neutral: bool,
    include_pending: bool,
    state: State<AppState>,
) -> ApiResult<billhub_core::LedgerSummary> {
    Ok(state.lock()?.summary(include_neutral, include_pending)?)
}

#[tauri::command]
fn api_monthly_summary(state: State<AppState>) -> ApiResult<Vec<billhub_core::PeriodSummary>> {
    Ok(state.lock()?.monthly_summary()?)
}

#[tauri::command]
fn api_yearly_summary(state: State<AppState>) -> ApiResult<Vec<billhub_core::PeriodSummary>> {
    Ok(state.lock()?.yearly_summary()?)
}

#[tauri::command]
fn api_month_dashboard(
    period: String,
    state: State<AppState>,
) -> ApiResult<Option<billhub_core::MonthDashboard>> {
    Ok(state.lock()?.month_dashboard(&period)?)
}

#[tauri::command]
fn api_delete(batch_id: String, state: State<AppState>) -> ApiResult<bool> {
    Ok(state.lock()?.delete_batch(&batch_id)?)
}

#[tauri::command]
async fn api_pick_statement(app: AppHandle) -> ApiResult<Option<String>> {
    let receiver = app
        .dialog()
        .file()
        .add_filter("账单文件", &["xlsx", "csv"])
        .blocking_pick_file();
    Ok(match receiver {
        Some(FilePath::Path(path)) => Some(path.to_string_lossy().into_owned()),
        Some(FilePath::Url(url)) => url.to_string().into(),
        None => None,
    })
}

#[tauri::command]
fn api_database_path(state: State<AppState>) -> String {
    state.database_path.to_string_lossy().into_owned()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_local_data_dir()?.join("ledger");
            std::fs::create_dir_all(&dir)?;
            let database_path = dir.join("billhub.sqlite3");
            let store = LedgerStore::open(&database_path)?;
            app.manage(AppState {
                database_path,
                store: Mutex::new(store),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api_preview,
            api_import,
            api_create_manual_entry,
            api_gamification,
            api_complete_daily_task,
            api_batches,
            api_events,
            api_update_event,
            api_update_events,
            api_summary,
            api_monthly_summary,
            api_yearly_summary,
            api_month_dashboard,
            api_delete,
            api_pick_statement,
            api_database_path
        ])
        .run(tauri::generate_context!())
        .expect("failed to run BillHub");
}
