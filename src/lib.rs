//! Классификатор заявок: загрузка правил, классификация, Excel-отчёты.
//!
//! Используется и консольным приложением (`main.rs`), и Tauri-GUI.

pub mod categories;
pub mod report;

use calamine::Reader;
use rust_xlsxwriter::{Format, Workbook, XlsxError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Результат классификации одного файла.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClassificationResult {
    /// Заголовки колонок исходного файла.
    pub headers: Vec<String>,
    /// Строки данных + присвоенная категория (последний элемент).
    pub rows: Vec<(Vec<String>, String)>,
    /// Количество заявок по категориям.
    pub category_count: HashMap<String, u32>,
    /// Имя обработанного файла.
    pub source_name: String,
}

impl ClassificationResult {
    /// Общее число классифицированных строк.
    pub fn total(&self) -> u32 {
        self.rows.len() as u32
    }
}

/// Ошибки библиотеки.
#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Xlsx(XlsxError),
    /// Файл не читается как Excel или не содержит нужных колонок.
    BadWorkbook(String),
    /// Ошибка конфигурации категорий.
    Config(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "ошибка ввода-вывода: {e}"),
            AppError::Xlsx(e) => write!(f, "ошибка записи xlsx: {e}"),
            AppError::BadWorkbook(msg) => write!(f, "{msg}"),
            AppError::Config(msg) => write!(f, "ошибка конфигурации: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<XlsxError> for AppError {
    fn from(e: XlsxError) -> Self {
        AppError::Xlsx(e)
    }
}

/// Классифицирует один Excel-файл.
///
/// Ищет колонку «Описание», применяет правила из `config` и возвращает
/// результат со строками и статистикой.
pub fn classify_file(
    path: &Path,
    config: &categories::CategoriesConfig,
) -> Result<ClassificationResult, AppError> {
    let mut input = calamine::open_workbook_auto(path)
        .map_err(|e| AppError::BadWorkbook(format!("не удалось открыть файл: {e}")))?;
    let sheet_name = input
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| AppError::BadWorkbook("документ пуст".into()))?;
    let range = input
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::BadWorkbook(format!("не удалось прочитать лист: {e}")))?;

    let mut rows = range.rows();
    let headers: Vec<String> = rows
        .next()
        .ok_or_else(|| AppError::BadWorkbook("файл пуст или не содержит строки заголовков".into()))?
        .iter()
        .map(|cell| cell.to_string())
        .collect();

    let desc_idx = headers
        .iter()
        .position(|h| h == "Описание")
        .ok_or_else(|| {
            AppError::BadWorkbook(format!(
                "колонка «Описание» не найдена в файле {}",
                path.display()
            ))
        })?;

    let mut category_count: HashMap<String, u32> = HashMap::new();
    let mut data_rows: Vec<(Vec<String>, String)> = Vec::new();

    for row in rows {
        let cells: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
        let description = cells.get(desc_idx).map(String::as_str).unwrap_or("");
        let category = config.classify(description);

        *category_count.entry(category.to_string()).or_insert(0) += 1;
        data_rows.push((cells, category));
    }

    Ok(ClassificationResult {
        headers,
        rows: data_rows,
        category_count,
        source_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    })
}

/// Сохраняет результат в Excel: лист с данными + лист «Статистика»
/// с таблицей и встроенными диаграммами.
pub fn save_report(
    result: &ClassificationResult,
    output_path: &Path,
) -> Result<(), AppError> {
    let mut workbook = Workbook::new();

    // --- Лист с данными ---
    let sheet = workbook.add_worksheet();
    sheet.set_name("Данные")?;

    let header_fmt = Format::new().set_bold();
    for (col, header) in result.headers.iter().enumerate() {
        sheet.write_with_format(0, col as u16, header.as_str(), &header_fmt)?;
    }
    let category_col = result.headers.len() as u16;
    sheet.write_with_format(0, category_col, "Категория", &header_fmt)?;
    sheet.autofit();

    for (i, (cells, category)) in result.rows.iter().enumerate() {
        let row_idx = (i + 1) as u32;
        for (col, cell) in cells.iter().enumerate() {
            sheet.write_string(row_idx, col as u16, cell.as_str())?;
        }
        sheet.write_string(row_idx, category_col, category.as_str())?;
    }
    sheet.set_freeze_panes(1, 0)?;

    report::add_statistics_sheet(&mut workbook, result)?;

    workbook.save(output_path)?;
    Ok(())
}

/// Путь к рабочему конфигу категорий.
///
/// Порядок поиска:
/// 1. `categories.toml` рядом с исполняемым файлом — портативная версия,
///    в неё же кладётся стандартный набор при сборке;
/// 2. `categories.toml` в текущей папке — удобно при запуске из репозитория;
/// 3. рядом с исполняемым файлом, если папка доступна для записи — так
///    portable-версия остаётся самодостаточной, даже если файл удалили;
/// 4. пользовательская папка настроек — для установленных сборок, где
///    каталог программы защищён от записи (`Program Files`, `/usr/bin`).
pub fn default_config_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));

    if let Some(dir) = &exe_dir {
        let candidate = dir.join("categories.toml");
        if candidate.exists() {
            return candidate;
        }
    }

    let cwd_candidate = PathBuf::from("categories.toml");
    if cwd_candidate.exists() {
        return cwd_candidate;
    }

    if let Some(dir) = &exe_dir {
        if is_writable_dir(dir) {
            return dir.join("categories.toml");
        }
    }

    user_config_dir().join("categories.toml")
}

/// Проверяет, можно ли создавать файлы в папке: проба создаётся и удаляется.
fn is_writable_dir(dir: &Path) -> bool {
    let probe = dir.join(format!(".rustifytickets-probe-{}", std::process::id()));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Пользовательская папка настроек приложения.
///
/// Windows: `%APPDATA%\RustifyTickets`, Linux/macOS: `$XDG_CONFIG_HOME`
/// или `~/.config/rustifytickets`.
pub fn user_config_dir() -> PathBuf {
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("RustifyTickets");
            }
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("rustifytickets");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".config").join("rustifytickets");
        }
    }
    PathBuf::from(".")
}

/// Записывает конфиг, создавая при необходимости родительские папки.
pub fn write_config(path: &Path, contents: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_writable_directory() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(
            is_writable_dir(&dir),
            "папка проекта должна быть доступна для записи"
        );
    }

    #[test]
    fn writability_probe_leaves_no_files() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let _ = is_writable_dir(&dir);
        let leftovers: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("папка проекта должна читаться")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".rustifytickets-probe-")
            })
            .map(|entry| entry.path())
            .collect();
        assert!(
            leftovers.is_empty(),
            "проба доступности записи не удалена: {leftovers:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn detects_read_only_directory() {
        // /proc существует, но создавать в нём файлы нельзя
        assert!(!is_writable_dir(Path::new("/proc")));
    }
}
