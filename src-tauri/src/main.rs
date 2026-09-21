// Скрывает лишнее окно консоли в релизной сборке Windows.
// В debug-сборке консоль остаётся — туда пишутся логи и отладочный вывод.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Tauri-команды GUI: загрузка файлов, конфиг категорий, отчёты.

use rustifytickets::categories::CategoriesConfig;
use rustifytickets::dates::{self, GroupBy};
use rustifytickets::{
    ClassificationResult, classify_file, filter_rows, period_counts, save_report_filtered,
};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{Emitter, State};

/// Состояние приложения: текущий конфиг категорий, путь и результаты последней классификации.
pub struct AppState {
    pub config: std::sync::Mutex<CategoriesConfig>,
    pub config_path: PathBuf,
    /// Файл пользовательского стандарта (`categories.default.toml`).
    /// Если он есть, «Сбросить к стандартным» восстанавливает именно его.
    pub default_config_path: PathBuf,
    pub last_results: std::sync::Mutex<Vec<ClassificationResult>>,
}

/// Результат сброса к стандартным правилам.
#[derive(Serialize)]
pub struct ResetOutcome {
    pub config: CategoriesConfig,
    /// "user" — пользовательский стандарт, "factory" — встроенный в программу.
    pub source: String,
}

/// Путь к файлу пользовательского стандарта — рядом с рабочим конфигом.
fn user_default_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|dir| dir.join("categories.default.toml"))
        .unwrap_or_else(|| PathBuf::from("categories.default.toml"))
}

/// Краткая сводка по одному обработанному файлу.
#[derive(Serialize, Clone)]
pub struct FileSummary {
    pub source_name: String,
    pub total: u32,
    /// Категория -> количество.
    pub category_count: HashMap<String, u32>,
    /// Колонка, из которой взяты даты заявок.
    pub date_column: Option<String>,
    /// Границы периода в формате `2026-07-01`.
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    /// Сколько заявок имеют распознанную дату.
    pub dated_rows: u32,
}

/// Точка динамики по периоду.
#[derive(Serialize, Clone)]
pub struct TimelinePoint {
    pub key: String,
    pub label: String,
    pub count: u32,
}

/// Данные вкладки «Графики» с учётом отбора по датам.
#[derive(Serialize)]
pub struct DatasetResponse {
    pub file_name: String,
    pub total_file: u32,
    pub total_filtered: u32,
    pub dated_rows: u32,
    pub date_column: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub group_by: String,
    /// Категория -> количество внутри отбора.
    pub category_count: HashMap<String, u32>,
    /// Динамика по месяцам или неделям.
    pub timeline: Vec<TimelinePoint>,
}

/// Ответ для постраничного просмотра данных классификации.
#[derive(Serialize)]
pub struct DataPageResponse {
    pub file_name: String,
    pub headers: Vec<String>,
    pub rows: Vec<(Vec<String>, String)>,
    pub total_filtered: usize,
    pub total_file: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    pub categories: Vec<String>,
    pub files: Vec<String>,
    /// Отбор по датам: сколько строк отсеяно из-за отсутствия даты.
    pub rows_without_date: usize,
    pub date_column: Option<String>,
}

/// Состояние на начало классификации (для прогресса).
#[derive(Serialize, Clone)]
pub struct ProgressEvent {
    pub processed: usize,
    pub total: usize,
    pub current_file: String,
}

/// Диалог выбора xlsx-файлов.
#[tauri::command]
async fn pick_input_files(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel::<Option<Vec<String>>>();
    app.dialog()
        .file()
        .add_filter("Excel файлы", &["xlsx", "xlsm", "xls"])
        .pick_files(move |paths| {
            let list = paths.map(|ps| {
                ps.into_iter()
                    .map(|p| p.into_path().unwrap_or_default().display().to_string())
                    .collect()
            });
            let _ = tx.send(list);
        });
    rx.recv()
        .map_err(|e| format!("ошибка диалога: {e}"))?
        .ok_or_else(|| "отменено".to_string())
}

/// Диалог сохранения отчёта: возвращает выбранный путь.
#[tauri::command]
async fn pick_save_path(app: tauri::AppHandle, default_name: String) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
    app.dialog()
        .file()
        .add_filter("Excel файл", &["xlsx"])
        .set_file_name(&default_name)
        .save_file(move |path| {
            let _ = tx.send(
                path.map(|p| p.into_path().unwrap_or_default().display().to_string()),
            );
        });
    rx.recv()
        .map_err(|e| format!("ошибка диалога: {e}"))?
        .ok_or_else(|| "отменено".to_string())
}

/// Классифицирует выбранные файлы, шлёт прогресс, сохраняет результаты в кэш и возвращает сводки.
#[tauri::command]
async fn classify_files(
    paths: Vec<String>,
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<Vec<FileSummary>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    let (summaries, all_results) = tauri::async_runtime::spawn_blocking(move || {
        let mut summaries = Vec::new();
        let mut all_results = Vec::new();
        let total = paths.len();
        for (i, path_str) in paths.iter().enumerate() {
            let path = Path::new(path_str);
            let _ = window.emit(
                "classify-progress",
                ProgressEvent {
                    processed: i,
                    total,
                    current_file: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                },
            );
            let result = classify_file(path, &config).map_err(|e| e.to_string())?;
            let source_name = result.source_name.clone();
            let file_total = result.total();
            let (date_from, date_to) = result.date_bounds();
            let dated_rows = result.dates.iter().filter(|d| d.is_some()).count() as u32;
            let _ = window.emit(
                "classify-progress",
                ProgressEvent {
                    processed: i + 1,
                    total,
                    current_file: source_name.clone(),
                },
            );
            summaries.push(FileSummary {
                source_name,
                total: file_total,
                category_count: result.category_count.clone(),
                date_column: result.date_column.clone(),
                date_from: date_from.map(|dt| dates::to_iso(dt.date())),
                date_to: date_to.map(|dt| dates::to_iso(dt.date())),
                dated_rows,
            });
            all_results.push(result);
        }
        Ok::<_, String>((summaries, all_results))
    })
    .await
    .map_err(|e| format!("ошибка задачи: {e}"))??;

    *state.last_results.lock().map_err(|e| e.to_string())? = all_results;
    Ok(summaries)
}

/// Постраничный просмотр классифицированных данных с фильтрацией по тексту и категории.
#[tauri::command]
fn get_classified_page(
    file_index: usize,
    page: usize,
    page_size: usize,
    search: String,
    category_filter: String,
    date_from: String,
    date_to: String,
    state: State<'_, AppState>,
) -> Result<DataPageResponse, String> {
    let results = state.last_results.lock().map_err(|e| e.to_string())?;
    if results.is_empty() {
        return Ok(DataPageResponse {
            file_name: String::new(),
            headers: Vec::new(),
            rows: Vec::new(),
            total_filtered: 0,
            total_file: 0,
            page: 1,
            page_size: page_size.max(1),
            total_pages: 0,
            categories: Vec::new(),
            files: Vec::new(),
            rows_without_date: 0,
            date_column: None,
        });
    }

    let idx = file_index.min(results.len().saturating_sub(1));
    let result = &results[idx];
    let file_name = result.source_name.clone();
    let total_file = result.rows.len();

    let search_trimmed = search.trim().to_lowercase();
    let cat_filter_trimmed = category_filter.trim();
    let date_filter = dates::filter_from_iso(&date_from, &date_to);

    let mut rows_without_date = 0usize;
    let mut filtered_rows: Vec<&(Vec<String>, String)> = Vec::new();

    for (index, row) in result.rows.iter().enumerate() {
        let date = result.dates.get(index).copied().flatten();

        // Отбор по периоду идёт первым: он же отсеивает строки без даты
        if !date_filter.is_empty() {
            if date.is_none() {
                rows_without_date += 1;
            }
            if !date_filter.accepts(date) {
                continue;
            }
        }

        let (cells, cat) = row;
        if !cat_filter_trimmed.is_empty() && cat != cat_filter_trimmed {
            continue;
        }
        if !search_trimmed.is_empty() {
            let in_cat = cat.to_lowercase().contains(&search_trimmed);
            let in_cells = cells
                .iter()
                .any(|c| c.to_lowercase().contains(&search_trimmed));
            if !in_cat && !in_cells {
                continue;
            }
        }
        filtered_rows.push(row);
    }

    let total_filtered = filtered_rows.len();
    let safe_page_size = page_size.max(1);
    let total_pages = ((total_filtered + safe_page_size - 1) / safe_page_size).max(1);
    let current_page = page.clamp(1, total_pages);

    let start = (current_page - 1) * safe_page_size;
    let end = (start + safe_page_size).min(total_filtered);

    let page_rows = if start < total_filtered {
        filtered_rows[start..end]
            .iter()
            .map(|r| (*r).clone())
            .collect()
    } else {
        Vec::new()
    };

    let mut categories: Vec<String> = result.category_count.keys().cloned().collect();
    categories.sort();

    let files: Vec<String> = results.iter().map(|r| r.source_name.clone()).collect();

    Ok(DataPageResponse {
        file_name,
        headers: result.headers.clone(),
        rows: page_rows,
        total_filtered,
        total_file,
        page: current_page,
        page_size: safe_page_size,
        total_pages,
        categories,
        files,
        rows_without_date,
        date_column: result.date_column.clone(),
    })
}

/// Данные для графиков: категории и динамика по периодам с учётом отбора.
#[tauri::command]
fn get_dataset(
    file_index: usize,
    date_from: String,
    date_to: String,
    group_by: String,
    state: State<'_, AppState>,
) -> Result<DatasetResponse, String> {
    let results = state.last_results.lock().map_err(|e| e.to_string())?;
    if results.is_empty() {
        return Ok(DatasetResponse {
            file_name: String::new(),
            total_file: 0,
            total_filtered: 0,
            dated_rows: 0,
            date_column: None,
            date_from: None,
            date_to: None,
            group_by,
            category_count: HashMap::new(),
            timeline: Vec::new(),
        });
    }

    let idx = file_index.min(results.len().saturating_sub(1));
    let result = &results[idx];
    let filter = dates::filter_from_iso(&date_from, &date_to);
    let group = GroupBy::from_str(&group_by);
    let filtered = filter_rows(result, &filter);
    let (bounds_from, bounds_to) = result.date_bounds();

    Ok(DatasetResponse {
        file_name: result.source_name.clone(),
        total_file: result.total(),
        total_filtered: filtered.rows.len() as u32,
        dated_rows: result.dates.iter().filter(|d| d.is_some()).count() as u32,
        date_column: result.date_column.clone(),
        date_from: bounds_from.map(|dt| dates::to_iso(dt.date())),
        date_to: bounds_to.map(|dt| dates::to_iso(dt.date())),
        group_by: match group {
            GroupBy::Month => "month".to_string(),
            GroupBy::Week => "week".to_string(),
        },
        category_count: filtered.category_count.clone(),
        timeline: period_counts(&filtered, group)
            .into_iter()
            .map(|point| TimelinePoint {
                key: point.key,
                label: point.label,
                count: point.count,
            })
            .collect(),
    })
}

/// Сохраняет отчёт по одному файлу в выбранный xlsx (с диаграммами).
///
/// `date_from`/`date_to` задают период в формате `2026-07-01`; пустая строка —
/// граница не ограничена. `group_by` — `month` или `week`.
#[tauri::command]
async fn save_report_file(
    input_path: String,
    output_path: String,
    date_from: String,
    date_to: String,
    group_by: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let group = GroupBy::from_str(&group_by);
    let cached_opt = {
        let guard = state.last_results.lock().map_err(|e| e.to_string())?;
        let p = Path::new(&input_path);
        let file_name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        guard.iter().find(|r| r.source_name == file_name).cloned()
    };

    tauri::async_runtime::spawn_blocking(move || {
        let result = if let Some(res) = cached_opt {
            res
        } else {
            classify_file(Path::new(&input_path), &config).map_err(|e| e.to_string())?
        };
        let filter = dates::filter_from_iso(&date_from, &date_to);
        save_report_filtered(&result, Path::new(&output_path), &filter, group)
            .map_err(|e| e.to_string())?;
        Ok(output_path)
    })
    .await
    .map_err(|e| format!("ошибка задачи: {e}"))?
}

/// Текущий конфиг категорий (для редактора в GUI).
#[tauri::command]
fn get_categories(state: State<'_, AppState>) -> Result<CategoriesConfig, String> {
    Ok(state.config.lock().map_err(|e| e.to_string())?.clone())
}

/// Сохраняет изменённый конфиг: в файл + в состояние.
#[tauri::command]
fn set_categories(
    config_toml: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config = CategoriesConfig::from_toml_str(&config_toml)?;
    rustifytickets::write_config(&state.config_path, &config_toml).map_err(|e| e.to_string())?;
    *state.config.lock().map_err(|e| e.to_string())? = config;
    Ok(())
}

/// Делает текущую конфигурацию стандартной: запоминает её как эталон,
/// к которому возвращает кнопка «Сбросить к стандартным».
#[tauri::command]
fn set_default_categories(
    config_toml: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config = CategoriesConfig::from_toml_str(&config_toml)?;
    rustifytickets::write_config(&state.default_config_path, &config_toml)
        .map_err(|e| e.to_string())?;
    rustifytickets::write_config(&state.config_path, &config_toml).map_err(|e| e.to_string())?;
    *state.config.lock().map_err(|e| e.to_string())? = config;
    Ok(())
}

/// Сброс к стандартным правилам: пользовательский стандарт, если он задан,
/// иначе встроенный в программу.
#[tauri::command]
fn reset_categories(state: State<'_, AppState>) -> Result<ResetOutcome, String> {
    let (config, toml_text, source) = if state.default_config_path.exists() {
        let text = std::fs::read_to_string(&state.default_config_path).map_err(|e| e.to_string())?;
        let config = CategoriesConfig::from_toml_str(&text)?;
        (config, text, "user")
    } else {
        (
            CategoriesConfig::defaults(),
            rustifytickets::categories::DEFAULT_CATEGORIES_TOML.to_string(),
            "factory",
        )
    };

    rustifytickets::write_config(&state.config_path, &toml_text).map_err(|e| e.to_string())?;
    *state.config.lock().map_err(|e| e.to_string())? = config.clone();
    Ok(ResetOutcome {
        config,
        source: source.to_string(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_path = rustifytickets::default_config_path();
    let default_config_path = user_default_path(&config_path);
    let config = CategoriesConfig::from_file(&config_path)
        .unwrap_or_else(|_| CategoriesConfig::defaults());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: std::sync::Mutex::new(config),
            config_path,
            default_config_path,
            last_results: std::sync::Mutex::new(Vec::new()),
        })
        .invoke_handler(tauri::generate_handler![
            pick_input_files,
            pick_save_path,
            classify_files,
            save_report_file,
            get_categories,
            set_categories,
            reset_categories,
            set_default_categories,
            get_classified_page,
            get_dataset,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска Tauri");
}

fn main() {
    run();
}
