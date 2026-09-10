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
    pub fn classify(&self, text: &str) -> String {
        let lower = text.trim().to_lowercase();

        let mut best: Option<(f32, u32, &str)> = None;
        for category in &self.categories {
            let excluded = category.exclude.iter().any(|word| lower.contains(word));
            if excluded {
                continue;
            }
            let score: f32 = category
                .include
                .iter()
                .filter(|word| lower.contains(word.as_str()))
                .map(|_| category.weight)
                .sum();

            if score > 0.0 && best.map_or(true, |(b_score, b_priority, _)| {
                score > b_score || (score == b_score && category.priority < b_priority)
            }) {
                best = Some((score, category.priority, &category.name));
            }
        }

        best.map(|(_, _, name)| name.to_string())
            .unwrap_or_else(|| UNCLASSIFIED.to_string())
    }
}

pub const UNCLASSIFIED: &str = "Не классифицировано";
