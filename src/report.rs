//! Лист «Статистика»: сводка по категориям, разбивка по периодам и диаграммы.

use crate::dates::{DateFilter, GroupBy};
use crate::{period_counts, ClassificationResult, FilteredRows};
use rust_xlsxwriter::{Chart, ChartType, Format, FormatBorder, Workbook, XlsxError};

/// Описание периода для шапки отчёта.
fn period_caption(filter: &DateFilter, group_by: GroupBy) -> String {
    let group = match group_by {
        GroupBy::Month => "по месяцам",
        GroupBy::Week => "по неделям",
    };
    match (filter.from, filter.to) {
        (Some(from), Some(to)) => format!(
            "Период: {} — {} (группировка {group})",
            from.format("%d.%m.%Y"),
            to.format("%d.%m.%Y")
        ),
        (Some(from), None) => format!("Период: с {} (группировка {group})", from.format("%d.%m.%Y")),
        (None, Some(to)) => format!("Период: по {} (группировка {group})", to.format("%d.%m.%Y")),
        (None, None) => format!("Период: весь файл (группировка {group})"),
    }
}

/// Добавляет лист «Статистика»: сводная таблица, разбивка по периодам
/// и диаграммы (кольцевая по категориям, гистограмма топ-10 и динамика).
pub fn add_statistics_sheet(
    workbook: &mut Workbook,
    result: &ClassificationResult,
    filtered: &FilteredRows<'_>,
    filter: &DateFilter,
    group_by: GroupBy,
) -> Result<(), XlsxError> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Статистика")?;

    let title_fmt = Format::new().set_bold().set_font_size(14);
    let header_fmt = Format::new().set_bold().set_border(FormatBorder::Thin);
    let cell_fmt = Format::new().set_border(FormatBorder::Thin);
    let pct_fmt = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("0.0%");

    // --- Шапка: что именно попало в отчёт ---
    sheet.write_with_format(0, 0, "Статистика по категориям", &title_fmt)?;
    sheet.write_with_format(1, 0, period_caption(filter, group_by), &Format::new())?;
    if let Some(column) = &result.date_column {
        sheet.write_with_format(2, 0, format!("Колонка даты: {column}"), &Format::new())?;
    }

    sheet.write_with_format(4, 0, "Категория", &header_fmt)?;
    sheet.write_with_format(4, 1, "Количество", &header_fmt)?;
    sheet.write_with_format(4, 2, "Доля", &header_fmt)?;

    // Сортировка по убыванию количества
    let mut stats: Vec<(&String, &u32)> = filtered.category_count.iter().collect();
    stats.sort_unstable_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    let total_count: u32 = stats.iter().map(|(_, count)| **count).sum();
    let total = total_count.max(1);

    let first_data_row = 5u32;
    for (i, (name, count)) in stats.iter().enumerate() {
        let row = first_data_row + i as u32;
        sheet.write_with_format(row, 0, name.as_str(), &cell_fmt)?;
        sheet.write_number_with_format(row, 1, **count as f64, &cell_fmt)?;
        sheet.write_number_with_format(row, 2, **count as f64 / total as f64, &pct_fmt)?;
    }
    let last_data_row = first_data_row + stats.len().saturating_sub(1) as u32;

    // Итоговая строка
    let total_row = last_data_row + 1;
    sheet.write_with_format(total_row, 0, "Итого за период", &header_fmt)?;
    sheet.write_number_with_format(total_row, 1, total_count as f64, &header_fmt)?;
    sheet.write_number_with_format(total_row, 2, 1.0, &pct_fmt)?;

    // --- Разбивка по периодам ---
    let periods = period_counts(filtered, group_by);
    let periods_title_row = total_row + 2;
    sheet.write_with_format(
        periods_title_row,
        0,
        match group_by {
            GroupBy::Month => "Заявки по месяцам",
            GroupBy::Week => "Заявки по неделям",
        },
        &title_fmt,
    )?;

    let period_header_row = periods_title_row + 1;
    sheet.write_with_format(period_header_row, 0, "Период", &header_fmt)?;
    sheet.write_with_format(period_header_row, 1, "Количество", &header_fmt)?;
    sheet.write_with_format(period_header_row, 2, "Доля", &header_fmt)?;

    let first_period_row = period_header_row + 1;
    for (i, period) in periods.iter().enumerate() {
        let row = first_period_row + i as u32;
        sheet.write_with_format(row, 0, period.label.as_str(), &cell_fmt)?;
        sheet.write_number_with_format(row, 1, period.count as f64, &cell_fmt)?;
        sheet.write_number_with_format(row, 2, period.count as f64 / total as f64, &pct_fmt)?;
    }
    let last_period_row = first_period_row + periods.len().saturating_sub(1) as u32;

    sheet.autofit();
    sheet.set_column_width(0, 42)?;

    // --- Кольцевая диаграмма: доли категорий ---
    if !stats.is_empty() {
        let mut doughnut = Chart::new(ChartType::Doughnut);
        doughnut
            .title()
            .set_name("Распределение заявок по категориям");
        doughnut
            .add_series()
            .set_categories(("Статистика", first_data_row, 0, last_data_row, 0))
            .set_values(("Статистика", first_data_row, 1, last_data_row, 1))
            .set_name("Количество");
        doughnut.set_height(440).set_width(560);
        sheet.insert_chart(first_data_row, 4, &doughnut)?;
    }

    // --- Гистограмма: топ-10 категорий ---
    let top_n = 10.min(stats.len()) as u32;
    if top_n > 0 {
        let mut bar = Chart::new(ChartType::Bar);
        bar.title().set_name("Топ-10 категорий по количеству заявок");
        bar.add_series()
            .set_categories(("Статистика", first_data_row, 0, first_data_row + top_n - 1, 0))
            .set_values(("Статистика", first_data_row, 1, first_data_row + top_n - 1, 1))
            .set_name("Количество");
        bar.set_height(440).set_width(560);
        sheet.insert_chart(first_data_row, 13, &bar)?;
    }

    // --- Динамика по периодам ---
    if !periods.is_empty() {
        let mut line = Chart::new(ChartType::Line);
        line.title().set_name(match group_by {
            GroupBy::Month => "Динамика заявок по месяцам",
            GroupBy::Week => "Динамика заявок по неделям",
        });
        line.add_series()
            .set_categories(("Статистика", first_period_row, 0, last_period_row, 0))
            .set_values(("Статистика", first_period_row, 1, last_period_row, 1))
            .set_name("Заявок за период");
        line.set_height(400).set_width(900);
        sheet.insert_chart(period_header_row + periods.len() as u32 + 2, 0, &line)?;
    }

    Ok(())
}
