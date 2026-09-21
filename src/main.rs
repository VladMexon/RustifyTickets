//! Консольный интерфейс классификатора заявок.

use rustifytickets::{categories, classify_file, default_config_path, save_report};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let input_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tickets.xlsx"));
    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("classified_tickets.xlsx"));

    let config_path = default_config_path();
    let config = if config_path.exists() {
        categories::CategoriesConfig::from_file(&config_path)?
    } else {
        eprintln!(
            "Конфиг {} не найден, используются встроенные правила",
            config_path.display()
        );
        categories::CategoriesConfig::defaults()
    };

    let result = classify_file(&input_path, &config)?;
    save_report(&result, &output_path)?;

    let mut stats: Vec<_> = result.category_count.iter().collect();
    stats.sort_unstable_by_key(|&(_, count)| std::cmp::Reverse(*count));
    for (name, count) in stats {
        println!("{name}: {count}");
    }

    // Что удалось понять про даты: по ним в GUI строится отбор за период
    match (&result.date_column, result.date_bounds()) {
        (Some(column), (Some(from), Some(to))) => println!(
            "\nДаты: колонка «{column}», период {} — {}",
            from.format("%d.%m.%Y"),
            to.format("%d.%m.%Y")
        ),
        (Some(column), _) => println!("\nДаты: колонка «{column}», значения не распознаны"),
        (None, _) => println!("\nДаты: колонка с датой не найдена"),
    }

    println!("\nГотово! Результат сохранен в: {}", output_path.display());

    Ok(())
}
