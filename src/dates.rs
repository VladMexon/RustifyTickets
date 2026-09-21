//! Разбор дат из выгрузок и группировка заявок по периодам.
//!
//! Выгрузки содержат даты как числовые серии Excel (`46204.000474537`), реже —
//! текстом. Модуль приводит и то и другое к [`NaiveDateTime`], а также умеет
//! группировать заявки по месяцам и неделям.

use calamine::Data;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};

/// Колонки с датой заявки в порядке предпочтения.
///
/// Основная — «Время создания»: именно по ней конечные пользователи отбирают
/// заявки за период.
pub const DATE_COLUMN_CANDIDATES: &[&str] = &[
    "Время создания",
    "Дата создания",
    "Время создания в OTRS",
    "Дата регистрации",
    "Создано",
];

/// Признаки того, что колонку стоит показывать как дату.
const DATE_HEADER_MARKERS: &[&str] = &["дата", "время", "срок", "date", "time", "created"];

/// Единица группировки заявок по времени.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    /// Календарный месяц.
    Month,
    /// Календарная неделя (ISO, с понедельника).
    Week,
}

impl GroupBy {
    /// Разбирает значение из интерфейса: `month` или `week`.
    pub fn from_str(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "week" | "неделя" | "недели" => GroupBy::Week,
            _ => GroupBy::Month,
        }
    }

    /// Ключ периода для сортировки: `2026-07` или `2026-W27`.
    pub fn key(&self, dt: NaiveDateTime) -> String {
        match self {
            GroupBy::Month => format!("{:04}-{:02}", dt.year(), dt.month()),
            GroupBy::Week => {
                let iso = dt.iso_week();
                format!("{:04}-W{:02}", iso.year(), iso.week())
            }
        }
    }

    /// Подпись периода для интерфейса и отчёта.
    pub fn label(&self, dt: NaiveDateTime) -> String {
        match self {
            GroupBy::Month => month_label(dt.date()),
            GroupBy::Week => week_label(dt.date()),
        }
    }
}

/// Диапазон дат, включительно по обеим границам.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DateFilter {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

impl DateFilter {
    /// Диапазон не задан — фильтровать нечего.
    pub fn is_empty(&self) -> bool {
        self.from.is_none() && self.to.is_none()
    }

    /// Подходит ли заявка под диапазон.
    ///
    /// Заявки с нераспознанной датой при заданном диапазоне не проходят: иначе
    /// в отчёт попадали бы строки, период которых проверить нельзя.
    pub fn accepts(&self, dt: Option<NaiveDateTime>) -> bool {
        if self.is_empty() {
            return true;
        }
        match dt {
            None => false,
            Some(dt) => {
                let date = dt.date();
                self.from.map_or(true, |from| date >= from)
                    && self.to.map_or(true, |to| date <= to)
            }
        }
    }
}

/// Ищет колонку с датой заявки среди заголовков.
pub fn find_date_column(headers: &[String]) -> Option<usize> {
    for candidate in DATE_COLUMN_CANDIDATES {
        if let Some(pos) = headers
            .iter()
            .position(|h| h.trim().eq_ignore_ascii_case(candidate))
        {
            return Some(pos);
        }
    }
    headers.iter().position(|h| looks_like_date_header(h))
}

/// Похож ли заголовок на дату — по нему форматируются значения в отчёте.
pub fn looks_like_date_header(header: &str) -> bool {
    let lower = header.to_lowercase();
    DATE_HEADER_MARKERS.iter().any(|marker| lower.contains(marker))
}

/// Разбирает ячейку выгрузки в дату и время.
pub fn parse_cell(cell: &Data) -> Option<NaiveDateTime> {
    match cell {
        Data::DateTime(dt) => serial_to_datetime(dt.as_f64()),
        Data::Int(value) => serial_to_datetime(*value as f64),
        Data::Float(value) => serial_to_datetime(*value),
        Data::String(value) | Data::DateTimeIso(value) => parse_text(value),
        _ => None,
    }
}

/// Разбирает дату, записанную текстом или числом в виде строки.
pub fn parse_text(value: &str) -> Option<NaiveDateTime> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    // «25.06.2026 14:35:07», «2026-06-25T14:35», «25/06/2026» и близкие варианты
    const FORMATS: &[&str] = &[
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M",
        "%d.%m.%Y",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d",
        "%d/%m/%Y %H:%M:%S",
        "%d/%m/%Y",
    ];
    for format in FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, format) {
            return Some(dt);
        }
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, format) {
            return date.and_hms_opt(0, 0, 0);
        }
    }

    // число, записанное текстом (например, «46204.000474537»)
    trimmed.parse::<f64>().ok().and_then(serial_to_datetime)
}

/// Переводит серию Excel (дни с 30.12.1899) в дату и время.
///
/// Для дат до 01.03.1900 учитывается «ошибка високосного года» Excel:
/// серия 1 — это 1900-01-01, а не 1899-12-31.
pub fn serial_to_datetime(serial: f64) -> Option<NaiveDateTime> {
    if !serial.is_finite() || serial < 1.0 || serial > 2_958_465.0 {
        return None; // вне диапазона 1900-01-01 … 9999-12-31
    }
    let mut days = serial.trunc() as i64;
    if days < 60 {
        days += 1; // компенсация несуществующего 29.02.1900
    }
    let seconds = (serial.fract() * 86_400.0).round() as i64;
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30)?.and_hms_opt(0, 0, 0)?;
    epoch
        .checked_add_signed(Duration::days(days))?
        .checked_add_signed(Duration::seconds(seconds))
}

/// Форматирует дату для отчёта: `01.07.2026 00:00:41`.
pub fn format_datetime(dt: NaiveDateTime) -> String {
    dt.format("%d.%m.%Y %H:%M:%S").to_string()
}

/// Разбирает дату из поля ввода интерфейса (`2026-07-01`).
pub fn parse_iso(value: &str) -> Option<NaiveDate> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok()
}

/// Форматирует дату для интерфейса (`2026-07-01`).
pub fn to_iso(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Собирает фильтр из значений полей ввода: пустая строка — граница не задана.
pub fn filter_from_iso(from: &str, to: &str) -> DateFilter {
    DateFilter {
        from: parse_iso(from),
        to: parse_iso(to),
    }
}

/// Форматирует значение колонки-даты, если оно распознаётся как дата.
pub fn format_date_value(value: &str) -> Option<String> {
    parse_text(value).map(format_datetime)
}

/// Подпись месяца: `июль 2026`.
pub fn month_label(date: NaiveDate) -> String {
    const MONTHS: [&str; 12] = [
        "январь",
        "февраль",
        "март",
        "апрель",
        "май",
        "июнь",
        "июль",
        "август",
        "сентябрь",
        "октябрь",
        "ноябрь",
        "декабрь",
    ];
    let month = MONTHS[(date.month0() as usize).min(11)];
    format!("{month} {}", date.year())
}

/// Подпись недели по её границам: `29.06–05.07.2026`.
pub fn week_label(date: NaiveDate) -> String {
    let start = date - Duration::days(date.weekday().num_days_from_monday() as i64);
    let end = start + Duration::days(6);
    format!(
        "{:02}.{:02}–{:02}.{:02}.{}",
        start.day(),
        start.month(),
        end.day(),
        end.month(),
        end.year()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(y: i32, m: u32, d: u32, hh: u32, mm: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(hh, mm, 0)
            .unwrap()
    }

    #[test]
    fn parses_excel_serial() {
        // 45000 — общеизвестная контрольная точка (15.03.2023)
        assert_eq!(serial_to_datetime(45000.0), Some(dt(2023, 3, 15, 0, 0)));
        // реальное значение из выгрузки: 01.07.2026 00:00:41
        let parsed = serial_to_datetime(46204.000474537).unwrap();
        assert_eq!(parsed.date(), NaiveDate::from_ymd_opt(2026, 7, 1).unwrap());
        assert_eq!(parsed.time(), chrono::NaiveTime::from_hms_opt(0, 0, 41).unwrap());
    }

    #[test]
    fn parses_serial_text_and_text_dates() {
        assert_eq!(parse_text("46204.000474537").unwrap().date(), NaiveDate::from_ymd_opt(2026, 7, 1).unwrap());
        assert_eq!(
            parse_text("25.06.2026 14:35:07"),
            NaiveDate::from_ymd_opt(2026, 6, 25)
                .unwrap()
                .and_hms_opt(14, 35, 7)
        );
        assert_eq!(parse_text("25.06.2026"), Some(dt(2026, 6, 25, 0, 0)));
        assert_eq!(parse_text("2026-06-25T09:15"), Some(dt(2026, 6, 25, 9, 15)));
        assert_eq!(parse_text("   "), None);
        assert_eq!(parse_text("нет даты"), None);
    }

    #[test]
    fn early_dates_skip_excel_leap_year_bug() {
        assert_eq!(serial_to_datetime(1.0), Some(dt(1900, 1, 1, 0, 0)));
        assert_eq!(serial_to_datetime(61.0), Some(dt(1900, 3, 1, 0, 0)));
    }

    #[test]
    fn rejects_implausible_serials() {
        assert_eq!(serial_to_datetime(0.0), None);
        assert_eq!(serial_to_datetime(-5.0), None);
        assert_eq!(serial_to_datetime(f64::NAN), None);
    }

    #[test]
    fn finds_date_column_by_priority() {
        let headers = vec!["Номер".to_string(), "Дата создания".to_string(), "Время создания".to_string()];
        assert_eq!(find_date_column(&headers), Some(2));
        let only_fallback = vec!["Номер".to_string(), "Крайний срок".to_string()];
        assert_eq!(find_date_column(&only_fallback), Some(1));
        let none = vec!["Номер".to_string(), "Описание".to_string()];
        assert_eq!(find_date_column(&none), None);
    }

    #[test]
    fn filter_is_inclusive_and_rejects_missing_dates() {
        let filter = DateFilter {
            from: Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
            to: Some(NaiveDate::from_ymd_opt(2026, 6, 30).unwrap()),
        };
        assert!(filter.accepts(Some(dt(2026, 6, 1, 0, 0))));
        assert!(filter.accepts(Some(dt(2026, 6, 30, 23, 59))));
        assert!(!filter.accepts(Some(dt(2026, 5, 31, 23, 59))));
        assert!(!filter.accepts(Some(dt(2026, 7, 1, 0, 0))));
        assert!(!filter.accepts(None));
        // пустой фильтр пропускает всё, включая строки без даты
        assert!(DateFilter::default().accepts(None));
    }

    #[test]
    fn groups_by_month_and_week() {
        let date = dt(2026, 7, 1, 12, 0); // 1 июля 2026, среда
        assert_eq!(GroupBy::Month.key(date), "2026-07");
        assert_eq!(GroupBy::Month.label(date), "июль 2026");
        assert_eq!(GroupBy::Week.key(date), "2026-W27");
        assert_eq!(GroupBy::Week.label(date), "29.06–05.07.2026");
        assert_eq!(GroupBy::from_str("week"), GroupBy::Week);
        assert_eq!(GroupBy::from_str("month"), GroupBy::Month);
        assert_eq!(GroupBy::from_str("что-то"), GroupBy::Month);
    }

    #[test]
    fn converts_iso_dates_for_interface() {
        assert_eq!(parse_iso("2026-07-01"), NaiveDate::from_ymd_opt(2026, 7, 1));
        assert_eq!(parse_iso(""), None);
        assert_eq!(parse_iso("01.07.2026"), None, "формат интерфейса — ISO");
        assert_eq!(to_iso(NaiveDate::from_ymd_opt(2026, 7, 1).unwrap()), "2026-07-01");

        let filter = filter_from_iso("2026-07-01", "");
        assert_eq!(filter.from, NaiveDate::from_ymd_opt(2026, 7, 1));
        assert_eq!(filter.to, None);
        assert!(filter_from_iso("", "").is_empty());
    }

    #[test]
    fn formats_date_column_values() {
        assert_eq!(
            format_date_value("46204.000474537").as_deref(),
            Some("01.07.2026 00:00:41")
        );
        assert_eq!(format_date_value("25.06.2026 14:35:07").as_deref(), Some("25.06.2026 14:35:07"));
        assert_eq!(format_date_value("не дата"), None);
    }
}
