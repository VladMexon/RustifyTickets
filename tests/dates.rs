//! Тесты отбора заявок по датам и отчёта за период.

use calamine::Reader;
use chrono::{NaiveDate, NaiveDateTime};
use rustifytickets::dates::{DateFilter, GroupBy};
use rustifytickets::{ClassificationResult, filter_rows, period_counts, save_report_filtered};
use std::collections::HashMap;

fn dt(year: i32, month: u32, day: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

/// Четыре заявки: июнь, две в июле (разные недели) и одна без даты.
fn sample() -> ClassificationResult {
    let headers = vec!["Время создания".to_string(), "Описание".to_string()];
    let rows = vec![
        (
            vec!["29.06.2026".into(), "wi-fi не работает".into()],
            "Связь".to_string(),
        ),
        (
            vec!["01.07.2026".into(), "не печатает принтер".into()],
            "ВТ".to_string(),
        ),
        (
            vec!["15.07.2026".into(), "wi-fi отвалился".into()],
            "Связь".to_string(),
        ),
        (
            vec!["".into(), "нет даты".into()],
            "Не классифицировано".to_string(),
        ),
    ];
    let dates = vec![
        Some(dt(2026, 6, 29)),
        Some(dt(2026, 7, 1)),
        Some(dt(2026, 7, 15)),
        None,
    ];
    let mut category_count = HashMap::new();
    category_count.insert("Связь".to_string(), 2);
    category_count.insert("ВТ".to_string(), 1);
    category_count.insert("Не классифицировано".to_string(), 1);

    ClassificationResult {
        headers,
        rows,
        dates,
        date_column: Some("Время создания".to_string()),
        category_count,
        source_name: "sample.xlsx".to_string(),
    }
}

#[test]
fn filter_keeps_only_rows_inside_range() {
    let result = sample();
    let filter = DateFilter {
        from: Some(date(2026, 7, 1)),
        to: Some(date(2026, 7, 31)),
    };
    let filtered = filter_rows(&result, &filter);

    assert_eq!(filtered.rows.len(), 2);
    assert_eq!(filtered.category_count.get("Связь"), Some(&1));
    assert_eq!(filtered.category_count.get("ВТ"), Some(&1));
    assert_eq!(filtered.category_count.get("Не классифицировано"), None);
}

#[test]
fn rows_without_date_are_dropped_when_range_is_set() {
    let result = sample();
    let filter = DateFilter {
        from: Some(date(2026, 1, 1)),
        to: None,
    };
    let filtered = filter_rows(&result, &filter);

    // строка без даты не проходит: период заявки проверить нельзя
    assert_eq!(filtered.rows.len(), 3);
    assert!(filtered.dates.iter().all(Option::is_some));
}

#[test]
fn empty_filter_keeps_everything() {
    let result = sample();
    let filtered = filter_rows(&result, &DateFilter::default());
    assert_eq!(filtered.rows.len(), 4);
}

#[test]
fn period_counts_are_chronological() {
    let result = sample();
    let filtered = filter_rows(&result, &DateFilter::default());

    let months = period_counts(&filtered, GroupBy::Month);
    assert_eq!(months.len(), 2);
    assert_eq!(months[0].key, "2026-06");
    assert_eq!(months[0].label, "июнь 2026");
    assert_eq!(months[0].count, 1);
    assert_eq!(months[1].key, "2026-07");
    assert_eq!(months[1].count, 2);

    let weeks = period_counts(&filtered, GroupBy::Week);
    // 29 июня — понедельник той же недели ISO, что и 1 июля; 15 июля — отдельная
    assert_eq!(weeks.len(), 2);
    assert_eq!(weeks[0].key, "2026-W27");
    assert_eq!(weeks[0].count, 2);
    assert_eq!(weeks[0].label, "29.06–05.07.2026");
    assert_eq!(weeks[1].key, "2026-W29");
    assert_eq!(weeks[1].count, 1);
}

#[test]
fn filtered_report_contains_only_selected_rows() {
    let result = sample();
    let filter = DateFilter {
        from: Some(date(2026, 7, 1)),
        to: Some(date(2026, 7, 31)),
    };
    let path = std::env::temp_dir().join(format!(
        "rustifytickets_report_{}.xlsx",
        std::process::id()
    ));

    save_report_filtered(&result, &path, &filter, GroupBy::Week).expect("отчёт должен сохраниться");

    let mut workbook = calamine::open_workbook_auto(&path).expect("отчёт должен читаться");
    let sheets = workbook.sheet_names().to_vec();
    assert_eq!(sheets, vec!["Данные".to_string(), "Статистика".to_string()]);

    let range = workbook.worksheet_range("Данные").unwrap();
    // строка заголовков + две отобранные заявки (июньская и строка без даты отсеяны)
    assert_eq!(range.height(), 3);
    // дата записана человеку понятным текстом, а не серией Excel
    let first_data_date = range.get_value((1, 0)).unwrap().to_string();
    assert_eq!(first_data_date, "01.07.2026 00:00:00");

    let stats = workbook.worksheet_range("Статистика").unwrap();
    let caption = stats.get_value((1, 0)).unwrap().to_string();
    assert!(
        caption.contains("01.07.2026") && caption.contains("по неделям"),
        "в шапке статистики должен быть период и группировка, получено: {caption}"
    );

    let _ = std::fs::remove_file(&path);
}
