//! Лист «Статистика» с таблицей категорий и встроенными диаграммами.

use crate::ClassificationResult;
use rust_xlsxwriter::{
    Chart, ChartType, Format, Workbook, XlsxError,
};

/// Добавляет лист «Статистика»: сводная таблица и две диаграммы
/// (кольцевая — доли категорий, гистограмма — топ-10 по количеству).
pub fn add_statistics_sheet(
    workbook: &mut Workbook,
    result: &ClassificationResult,
) -> Result<(), XlsxError> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Статистика")?;

    let title_fmt = Format::new().set_bold().set_font_size(14);
    let header_fmt = Format::new().set_bold().set_border(rust_xlsxwriter::FormatBorder::Thin);
    let cell_fmt = Format::new().set_border(rust_xlsxwriter::FormatBorder::Thin);
    let pct_fmt = Format::new()
        .set_border(rust_xlsxwriter::FormatBorder::Thin)
        .set_num_format("0.0%");

    sheet.write_with_format(0, 0, "Статистика по категориям", &title_fmt)?;
    sheet.write_with_format(2, 0, "Категория", &header_fmt)?;
    sheet.write_with_format(2, 1, "Количество", &header_fmt)?;
    sheet.write_with_format(2, 2, "Доля", &header_fmt)?;

    // Сортировка по убыванию количества
    let mut stats: Vec<(&String, &u32)> = result.category_count.iter().collect();
    stats.sort_unstable_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    let total = result.total().max(1);
    let first_data_row = 3u32;
    for (i, (name, count)) in stats.iter().enumerate() {
        let row = first_data_row + i as u32;
        sheet.write_with_format(row, 0, name.as_str(), &cell_fmt)?;
        sheet.write_number_with_format(row, 1, **count as f64, &cell_fmt)?;
        sheet.write_number_with_format(row, 2, **count as f64 / total as f64, &pct_fmt)?;
    }
    let last_data_row = first_data_row + stats.len() as u32 - 1;

    // Итоговая строка
    let total_row = last_data_row + 1;
    sheet.write_with_format(total_row, 0, "Итого", &header_fmt)?;
    sheet.write_number_with_format(total_row, 1, result.total() as f64, &header_fmt)?;
    sheet.write_number_with_format(total_row, 2, 1.0, &pct_fmt)?;

    sheet.autofit();
    sheet.set_column_width(0, 40)?;

    // --- Кольцевая диаграмма: доли категорий ---
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
    sheet.insert_chart(3, 0, &doughnut)?;

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
        sheet.insert_chart(3, 9, &bar)?;
    }

    Ok(())
}
