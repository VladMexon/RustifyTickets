//! Загрузка и применение правил классификации из `categories.toml`.

use serde::Deserialize;
use std::path::Path;

/// Один список правил: вектор категорий + версия.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct CategoriesConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    // В JSON поле называется `categories`; в TOML массив таблиц называется
    // `[[category]]`, поэтому для обратной совместимости добавлен alias.
    #[serde(default, alias = "category")]
    pub categories: Vec<CategoryRule>,
}

fn default_version() -> u32 {
    1
}

/// Правило классификации: имя категории, списки слов, вес и приоритет.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct CategoryRule {
    pub name: String,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default = "default_weight")]
    pub weight: f32,
    /// Приоритет при равенстве весов: меньше число = выше приоритет.
    #[serde(default)]
    pub priority: u32,
}

fn default_weight() -> f32 {
    1.0
}

/// Встроенные правила по умолчанию (используются для «отката к стандартным»).
pub const DEFAULT_CATEGORIES_TOML: &str = include_str!("../categories.toml");

impl CategoriesConfig {
    /// Загружает конфиг из TOML-строки.
    pub fn from_toml_str(toml_str: &str) -> Result<Self, String> {
        let config: CategoriesConfig = toml::from_str(toml_str)
            .map_err(|e| format!("не удалось разобрать categories.toml: {e}"))?;
        config.validate()?;
        Ok(config)
    }

    /// Загружает конфиг из файла.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("не удалось прочитать {}: {e}", path.display()))?;
        Self::from_toml_str(&content)
    }

    /// Возвращает копию встроенных правил по умолчанию.
    pub fn defaults() -> Self {
        Self::from_toml_str(DEFAULT_CATEGORIES_TOML).expect("встроенный конфиг валиден")
    }

    /// Проверяет корректность конфига.
    pub fn validate(&self) -> Result<(), String> {
        if self.categories.is_empty() {
            return Err("список категорий пуст".into());
        }
        let mut seen = std::collections::HashSet::new();
        for cat in &self.categories {
            if !seen.insert(&cat.name) {
                return Err(format!("дублирующееся имя категории: «{}»", cat.name));
            }
        }
        Ok(())
    }

    /// Возвращает имя наиболее подходящей категории или `UNCLASSIFIED`.
    ///
    /// Категория подходит, если текст содержит хотя бы одно слово из
    /// `include` и ни одного из `exclude`. Побеждает категория с наибольшей
    /// суммой весов совпавших слов; при равенстве — с меньшим `priority`.
    ///
    /// Удобная обёртка: готовит правила на один вызов. Для обработки файла
    /// используйте [`CategoriesConfig::matcher`], чтобы не нормализовать
    /// слова для каждой строки заново.
    pub fn classify(&self, text: &str) -> String {
        self.matcher().classify(text).to_string()
    }

    /// Готовит правила к сопоставлению: приводит слова к нижнему регистру и
    /// унифицирует разделители. Результат переиспользуется для всех строк файла.
    pub fn matcher(&self) -> CategoryMatcher {
        CategoryMatcher {
            rules: self
                .categories
                .iter()
                .map(|category| CompiledRule {
                    name: category.name.clone(),
                    include: compile_words(&category.include),
                    exclude: compile_words(&category.exclude),
                    weight: category.weight,
                    priority: category.priority,
                })
                .collect(),
        }
    }
}

/// Нормализует слова и убирает дубликаты.
///
/// Дубликаты появляются после нормализации: `z-отчет` и `z отчет` дают одну
/// и ту же строку. Без дедупликации такое слово учитывалось бы в счёте дважды
/// и неоправданно повышало категорию.
fn compile_words(words: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for word in words {
        let normalized = normalize(word);
        if normalized.is_empty() || out.contains(&normalized) {
            continue;
        }
        out.push(normalized);
    }
    out
}

/// Правила, подготовленные к сопоставлению: слова уже нормализованы.
///
/// Нормализация нужна, потому что регистр и вид разделителя в правилах и в
/// тексте заявки не совпадают: пользователь пишет `Wi-Fi` или `VPN`, а в
/// выгрузке встречаются `wi-fi`, `wi–fi` (тире), `wi‑fi` (неразрывный дефис),
/// `wi fi` (пробел). Сравнение идёт по нормализованным строкам.
#[derive(Debug, Clone)]
pub struct CategoryMatcher {
    rules: Vec<CompiledRule>,
}

#[derive(Debug, Clone)]
struct CompiledRule {
    name: String,
    include: Vec<String>,
    exclude: Vec<String>,
    weight: f32,
    priority: u32,
}

impl CategoryMatcher {
    /// Возвращает имя подходящей категории или [`UNCLASSIFIED`].
    pub fn classify(&self, text: &str) -> &str {
        let normalized = normalize(text);

        let mut best: Option<(f32, u32, &str)> = None;
        for rule in &self.rules {
            if rule
                .exclude
                .iter()
                .any(|word| !word.is_empty() && normalized.contains(word.as_str()))
            {
                continue;
            }

            let score: f32 = rule
                .include
                .iter()
                .filter(|word| !word.is_empty() && normalized.contains(word.as_str()))
                .map(|_| rule.weight)
                .sum();

            if score > 0.0
                && best.map_or(true, |(b_score, b_priority, _)| {
                    score > b_score || (score == b_score && rule.priority < b_priority)
                })
            {
                best = Some((score, rule.priority, &rule.name));
            }
        }

        best.map(|(_, _, name)| name).unwrap_or(UNCLASSIFIED)
    }
}

/// Приводит текст и слова правил к единому виду.
///
/// - нижний регистр (важно для английских слов: `Wi-Fi`, `VPN`, `ORA`);
/// - все виды дефисов и тире (`-`, `–`, `—`, `‑`, минус) → пробел;
/// - неразрывные и узкие пробелы, табы, переводы строк → обычный пробел;
/// - повторяющиеся пробелы схлопываются в один.
///
/// Дефис приравнивается к пробелу, поэтому правило `wi-fi` находит и `wi-fi`,
/// и `wi–fi`, и `wi fi`. Обратное неверно: правило `wi fi` не найдёт `wifi`
/// без разделителя — для слитных написаний добавляйте отдельное слово.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;

    for ch in text.chars() {
        let mapped = match ch {
            // ASCII-дефис, дефисы, тире, минус и мягкий перенос
            '-' | '\u{2010}'..='\u{2015}' | '\u{2212}' | '\u{00AD}' | '\u{FE58}' | '\u{FE63}'
            | '\u{FF0D}' => ' ',
            // неразрывные и «тонкие» пробелы
            '\u{00A0}' | '\u{202F}' | '\u{2007}' | '\u{2009}' | '\u{200A}' => ' ',
            c if c.is_whitespace() => ' ',
            c => c,
        };

        for lower in mapped.to_lowercase() {
            if lower == ' ' {
                if prev_space {
                    continue;
                }
                prev_space = true;
            } else {
                prev_space = false;
            }
            out.push(lower);
        }
    }

    out.trim().to_string()
}

pub const UNCLASSIFIED: &str = "Не классифицировано";

#[cfg(test)]
mod tests {
    use super::*;

    /// Конфиг из одной категории с заданными словами.
    fn config_with(include: &[&str], exclude: &[&str]) -> CategoriesConfig {
        CategoriesConfig {
            version: 2,
            categories: vec![CategoryRule {
                name: "Тест".to_string(),
                include: include.iter().map(|w| w.to_string()).collect(),
                exclude: exclude.iter().map(|w| w.to_string()).collect(),
                weight: 1.0,
                priority: 0,
            }],
        }
    }

    fn matches(include: &[&str], text: &str) -> bool {
        config_with(include, &[]).classify(text) == "Тест"
    }

    #[test]
    fn uppercase_rule_matches_lowercase_text() {
        // Так было сломано правило ORA: слово в верхнем регистре не совпадало
        assert!(matches(&["ORA"], "ошибка ora на кассе"));
        assert!(matches(&["Wi-Fi"], "не работает wi-fi"));
        assert!(matches(&["VPN"], "не поднимается vpn-туннель"));
        assert!(matches(&["Excel"], "не открывается excel"));
    }

    #[test]
    fn mixed_case_rule_matches_mixed_case_text() {
        assert!(matches(&["ViPNet"], "Проблема с ViPNet Клиент"));
        assert!(matches(&["Wi-Fi"], "Wi-Fi не работает"));
    }

    #[test]
    fn dash_variants_are_equivalent_to_space() {
        let texts = [
            "не работает wi-fi",   // ASCII-дефис
            "не работает wi–fi",   // en dash
            "не работает wi—fi",   // em dash
            "не работает wi‑fi",   // неразрывный дефис
            "не работает wi fi",   // пробел
            "не работает wi\u{00a0}fi", // неразрывный пробел
        ];
        for text in texts {
            assert!(matches(&["wi-fi"], text), "не найдено в {text:?}");
        }
    }

    #[test]
    fn dash_in_rule_is_normalized_too() {
        // правило записано через тире, текст — через обычный дефис
        assert!(matches(&["b2b–обмен"], "ошибка b2b-обмен"));
        assert!(matches(&["z–отчет"], "не формируется z-отчет"));
    }

    #[test]
    fn solid_spelling_needs_its_own_word() {
        // «wifi» и «wi-fi» — разные написания, нужны оба слова в правилах
        assert!(!matches(&["wi-fi"], "пропал wifi"));
        assert!(matches(&["wi-fi", "wifi"], "пропал wifi"));
    }

    #[test]
    fn exclude_is_also_case_insensitive() {
        let config = config_with(&["wi-fi"], &["Отчет"]);
        assert_eq!(config.classify("wi-fi не работает"), "Тест");
        assert_eq!(config.classify("wi-fi в отчете"), UNCLASSIFIED);
    }

    #[test]
    fn normalize_collapses_spaces_and_lowercases() {
        assert_eq!(normalize("  Wi-Fi   Не\u{00a0}Работает  "), "wi fi не работает");
        assert_eq!(normalize("A–B—C‑D"), "a b c d");
    }

    #[test]
    fn weights_and_priority_still_work() {
        let config = CategoriesConfig {
            version: 2,
            categories: vec![
                CategoryRule {
                    name: "Слабый".into(),
                    include: vec!["wi fi".into()],
                    exclude: vec![],
                    weight: 1.0,
                    priority: 0,
                },
                CategoryRule {
                    name: "Сильный".into(),
                    include: vec!["wi fi".into(), "роутер".into()],
                    exclude: vec![],
                    weight: 1.0,
                    priority: 5,
                },
            ],
        };
        assert_eq!(config.classify("wi-fi роутер не работает"), "Сильный");
        // при равных весах решает меньший приоритет
        assert_eq!(config.classify("wi-fi не работает"), "Слабый");
    }

    #[test]
    fn normalized_duplicates_are_counted_once() {
        // «z-отчет» и «z отчет» после нормализации — одно и то же слово
        let config = CategoriesConfig {
            version: 2,
            categories: vec![
                CategoryRule {
                    name: "Дубли".into(),
                    include: vec!["z-отчет".into(), "z отчет".into()],
                    exclude: vec![],
                    weight: 1.0,
                    priority: 0,
                },
                CategoryRule {
                    name: "Соперник".into(),
                    include: vec!["касс".into(), "ошибка".into()],
                    exclude: vec![],
                    weight: 1.0,
                    priority: 9,
                },
            ],
        };
        // дубликат нормализованного слова даёт 1 балл, а не 2: без дедупликации
        // «Дубли» набрали бы 2 балла и выиграли по приоритету
        assert_eq!(config.classify("z-отчет и ошибка кассы"), "Соперник");
        assert_eq!(config.classify("не сформирован z-отчет"), "Дубли");
    }
}
