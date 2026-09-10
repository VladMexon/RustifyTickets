//! Проверки встроенного стандартного набора правил классификации.

use rustifytickets::categories::CategoriesConfig;
use std::path::PathBuf;

/// Встроенный (заводской) стандарт должен быть валиден и непуст.
#[test]
fn embedded_defaults_are_valid() {
    let config = CategoriesConfig::defaults();
    assert!(
        config.categories.len() >= 20,
        "ожидалось не меньше 20 категорий, получено {}",
        config.categories.len()
    );
    assert!(
        config.validate().is_ok(),
        "встроенный конфиг не проходит валидацию: {:?}",
        config.validate()
    );
}

/// Ключевые слова, добавленные в ходе настройки, должны остаться в стандарте.
#[test]
fn embedded_defaults_contain_tuned_keywords() {
    let config = CategoriesConfig::defaults();

    let expected: &[(&str, &[&str])] = &[
        (
            "Петроникс - Сменный отчет",
            &["сформировано", "переданного", "открытие", "закрытие"],
        ),
        ("Метрологическое оборудование", &["зачистка"]),
        ("ТО компьютерного оборудования", &["оборудования", "замена"]),
        ("Ошибки КАСУ - ТРК", &["доза"]),
        ("Связь", &["не проход"]),
        ("Медиаконтент", &["реклама", "повар"]),
        ("ВТ", &["почты"]),
        (
            "Ошибки КАСУ - общее",
            &["не могут", "не может", "не могу", "возврат"],
        ),
    ];

    for (cat_name, words) in expected {
        let cat = config
            .categories
            .iter()
            .find(|c| &c.name == cat_name)
            .unwrap_or_else(|| panic!("категория «{cat_name}» отсутствует в стандарте"));
        for word in *words {
            assert!(
                cat.include.iter().any(|w| w == word),
                "в категории «{cat_name}» нет слова «{word}»"
            );
        }
    }
}

/// «Нефтебазы» должны иметь приоритет 0 — иначе ничьи проигрываются.
#[test]
fn neftebazy_priority_is_zero() {
    let config = CategoriesConfig::defaults();
    let cat = config
        .categories
        .iter()
        .find(|c| c.name == "Нефтебазы")
        .expect("категория «Нефтебазы» отсутствует");
    assert_eq!(cat.priority, 0, "приоритет «Нефтебазы» должен быть 0");
}

/// Рабочий конфиг и встроенный стандарт должны совпадать: если файл
/// `src-tauri/categories.toml` есть, значит стандарт уже синхронизирован.
#[test]
fn embedded_defaults_match_working_config_if_present() {
    let working = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src-tauri/categories.toml");
    if !working.exists() {
        return; // чистая сборка без рабочего конфига — проверять нечего
    }
    let working_config = CategoriesConfig::from_file(&working)
        .expect("рабочий конфиг должен разбираться");
    let embedded = CategoriesConfig::defaults();

    assert_eq!(
        working_config.categories.len(),
        embedded.categories.len(),
        "число категорий в стандарте и рабочем конфиге различается"
    );
    for (working_cat, embedded_cat) in working_config
        .categories
        .iter()
        .zip(embedded.categories.iter())
    {
        assert_eq!(working_cat.name, embedded_cat.name, "порядок категорий разошёлся");
        assert_eq!(
            working_cat.include, embedded_cat.include,
            "слова-включения категории «{}» разошлись со стандартом",
            working_cat.name
        );
        assert_eq!(
            working_cat.exclude, embedded_cat.exclude,
            "слова-исключения категории «{}» разошлись со стандартом",
            working_cat.name
        );
        assert_eq!(
            working_cat.priority, embedded_cat.priority,
            "приоритет категории «{}» разошёлся со стандартом",
            working_cat.name
        );
    }
}
