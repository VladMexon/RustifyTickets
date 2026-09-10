// Редактор списков ключевых слов с откатом к стандартным.
import {
  getCategories,
  resetCategories,
  setCategories,
  setDefaultCategories,
  type CategoriesConfig,
  type CategoryRule,
} from "./api";

let config: CategoriesConfig | null = null;
let search = "";
let expanded = new Set<string>();

const listEl = () => document.getElementById("category-list") as HTMLDivElement;
const statusEl = () => document.getElementById("editor-status") as HTMLDivElement;

export async function initEditor() {
  const searchBox = document.getElementById("cat-search") as HTMLInputElement;
  const saveBtn = document.getElementById("save-cats") as HTMLButtonElement;
  const resetBtn = document.getElementById("reset-cats") as HTMLButtonElement;
  const addBtn = document.getElementById("add-cat") as HTMLButtonElement;
  const expandAllBtn = document.getElementById("cat-expand-all") as HTMLButtonElement;
  const collapseAllBtn = document.getElementById("cat-collapse-all") as HTMLButtonElement;
  const makeDefaultBtn = document.getElementById("make-default-cats") as HTMLButtonElement;

  searchBox.addEventListener("input", () => {
    search = searchBox.value.trim().toLowerCase();
    render();
  });

  saveBtn.addEventListener("click", async () => {
    try {
      await setCategories(toToml(config!)); // config всегда задан к этому моменту
      setStatus("Конфиг сохранён", "ok");
    } catch (e) {
      setStatus(String(e), "error");
    }
  });

  resetBtn.addEventListener("click", async () => {
    if (!confirm("Сбросить все категории к стандартным? Текущие изменения будут потеряны.")) return;
    try {
      const outcome = await resetCategories();
      config = outcome.config;
      render();
      setStatus(
        outcome.source === "user"
          ? "Восстановлены ваши стандартные категории"
          : "Восстановлены встроенные стандартные категории",
        "ok",
      );
    } catch (e) {
      setStatus(String(e), "error");
    }
  });

  makeDefaultBtn.addEventListener("click", async () => {
    if (!config) return;
    if (
      !confirm(
        "Сделать текущую конфигурацию стандартной?\n\n" +
          "Эти правила будут восстанавливаться кнопкой «Сбросить к стандартным».",
      )
    ) {
      return;
    }
    try {
      await setDefaultCategories(toToml(config));
      setStatus("Текущая конфигурация сохранена как стандартная", "ok");
    } catch (e) {
      setStatus(String(e), "error");
    }
  });

  addBtn.addEventListener("click", () => {
    if (!config) return;
    const cat: CategoryRule = {
      name: "Новая категория",
      include: [],
      exclude: [],
      weight: 1,
      // приоритет 0: новые категории выигрывают ничью против стандартных,
      // иначе они не срабатывают из-за более высокого priority старых
      priority: 0,
    };
    config.categories.push(cat);
    expanded.add(cat.name);
    render();
    // Прокрутить к новой категории и сфокусировать имя
    const items = listEl().querySelectorAll(".category-item");
    const last = items[items.length - 1];
    last?.scrollIntoView({ behavior: "smooth", block: "center" });
    (last?.querySelector(".cat-name") as HTMLInputElement)?.focus();
    (last?.querySelector(".cat-name") as HTMLInputElement)?.select();
    setStatus("Категория добавлена. Не забудьте нажать «Сохранить».", "");
  });

  expandAllBtn.addEventListener("click", () => {
    if (!config) return;
    expanded = new Set(config.categories.map((c) => c.name));
    render();
  });

  collapseAllBtn.addEventListener("click", () => {
    expanded.clear();
    render();
  });

  try {
    config = await getCategories();
    if (!Array.isArray(config.categories)) {
      throw new Error("backend вернул конфиг без списка категорий");
    }
    render();
  } catch (e) {
    setStatus(String(e), "error");
  }
}

function render() {
  const list = listEl();
  list.innerHTML = "";
  const cats = (config?.categories ?? []).filter((c) =>
    search
      ? c.name.toLowerCase().includes(search) ||
        c.include.some((w) => w.toLowerCase().includes(search)) ||
        c.exclude.some((w) => w.toLowerCase().includes(search))
      : true,
  );

  for (const cat of cats) {
    list.appendChild(renderCategory(cat));
  }

  const countEl = document.getElementById("cat-count");
  if (countEl) {
    countEl.textContent = search
      ? `Найдено: ${cats.length} из ${config?.categories.length ?? 0}`
      : `Всего категорий: ${config?.categories.length ?? 0}`;
  }
}

function renderCategory(cat: CategoryRule): HTMLElement {
  const item = document.createElement("div");
  item.className = "category-item";
  const isOpen = expanded.has(cat.name);

  const head = document.createElement("div");
  head.className = "cat-head";

  const toggle = document.createElement("button");
  toggle.className = "cat-toggle";
  toggle.textContent = isOpen ? "▾" : "▸";
  toggle.title = isOpen ? "Свернуть" : "Развернуть";
  toggle.addEventListener("click", () => {
    if (expanded.has(cat.name)) expanded.delete(cat.name);
    else expanded.add(cat.name);
    render();
  });

  const nameInput = document.createElement("input");
  nameInput.className = "cat-name";
  nameInput.value = cat.name;
  nameInput.addEventListener("change", () => {
    if (expanded.has(cat.name)) {
      expanded.delete(cat.name);
      expanded.add(nameInput.value);
    }
    cat.name = nameInput.value;
  });

  // Мини-статистика: количество слов
  const stat = document.createElement("span");
  stat.className = "cat-stat";
  stat.textContent = `${cat.include.length} сл.`;
  stat.title = `Слов-включений: ${cat.include.length}, исключений: ${cat.exclude.length}`;

  const delBtn = document.createElement("button");
  delBtn.className = "danger";
  delBtn.textContent = "Удалить";
  delBtn.addEventListener("click", () => {
    if (!confirm(`Удалить категорию «${cat.name}»?`)) return;
    config!.categories = config!.categories.filter((c) => c !== cat);
    render();
  });

  head.append(toggle, nameInput, stat, delBtn);

  // Клик по заголовку (не по имени и не кнопкам) тоже разворачивает карточку
  head.addEventListener("click", (e) => {
    const t = e.target as HTMLElement;
    if (t.closest("input, button, .cat-stat")) return;
    if (expanded.has(cat.name)) expanded.delete(cat.name);
    else expanded.add(cat.name);
    render();
  });

  const body = document.createElement("div");
  body.className = "cat-body";
  body.style.display = isOpen ? "" : "none";

  const editor = document.createElement("div");
  editor.className = "words-editor";

  const incLabel = document.createElement("div");
  incLabel.className = "field-label";
  incLabel.textContent = "Слова-включения (через запятую)";
  const incArea = document.createElement("textarea");
  incArea.value = cat.include.join(", ");
  incArea.rows = Math.min(8, Math.max(2, Math.ceil(cat.include.length / 4)));
  incArea.addEventListener("change", () => {
    cat.include = splitWords(incArea.value);
    stat.textContent = `${cat.include.length} сл.`;
    stat.title = `Слов-включений: ${cat.include.length}, исключений: ${cat.exclude.length}`;
  });

  const excLabel = document.createElement("div");
  excLabel.className = "field-label";
  excLabel.textContent = "Слова-исключения (через запятую)";
  const excArea = document.createElement("textarea");
  excArea.value = cat.exclude.join(", ");
  excArea.addEventListener("change", () => {
    cat.exclude = splitWords(excArea.value);
    stat.title = `Слов-включений: ${cat.include.length}, исключений: ${cat.exclude.length}`;
  });

  editor.append(incLabel, incArea, excLabel, excArea);

  // Параметры взвешивания
  const params = document.createElement("div");
  params.className = "cat-params";

  const weightWrap = document.createElement("label");
  weightWrap.className = "param-field";
  weightWrap.innerHTML = `<span title="Вес одного совпавшего слова. Побеждает категория с наибольшей суммой весов">Вес</span>`;
  const weightInput = document.createElement("input");
  weightInput.type = "number";
  weightInput.step = "0.5";
  weightInput.min = "0.5";
  weightInput.value = String(cat.weight);
  weightInput.addEventListener("change", () => {
    const v = parseFloat(weightInput.value);
    cat.weight = Number.isFinite(v) && v > 0 ? v : 1;
    weightInput.value = String(cat.weight);
  });
  weightWrap.appendChild(weightInput);

  const prioWrap = document.createElement("label");
  prioWrap.className = "param-field";
  prioWrap.innerHTML = `<span title="Приоритет при равенстве весов: меньше число = выше приоритет">Приоритет</span>`;
  const prioInput = document.createElement("input");
  prioInput.type = "number";
  prioInput.step = "1";
  prioInput.value = String(cat.priority);
  prioInput.addEventListener("change", () => {
    const v = parseInt(prioInput.value, 10);
    cat.priority = Number.isFinite(v) && v >= 0 ? v : 0;
    prioInput.value = String(cat.priority);
  });
  prioWrap.appendChild(prioInput);

  params.append(weightWrap, prioWrap);
  body.append(editor, params);
  item.append(head, body);
  return item;
}

function splitWords(text: string): string[] {
  return text
    .split(",")
    .map((w) => w.trim())
    .filter((w) => w.length > 0);
}

function toToml(cfg: CategoriesConfig): string {
  const esc = (s: string) => s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  const lines: string[] = [`version = ${cfg.version}`, ""];
  for (const cat of cfg.categories) {
    lines.push("[[category]]");
    lines.push(`name = "${esc(cat.name)}"`);
    lines.push(`priority = ${cat.priority}`);
    if (cat.weight !== 1) {
      lines.push(`weight = ${cat.weight}`);
    }
    lines.push(
      `include = [${cat.include.map((w) => `"${esc(w)}"`).join(", ")}]`,
    );
    lines.push(
      `exclude = [${cat.exclude.map((w) => `"${esc(w)}"`).join(", ")}]`,
    );
    lines.push("");
  }
  return lines.join("\n");
}

function setStatus(text: string, cls: string) {
  const el = statusEl();
  el.textContent = text;
  el.className = `status-line ${cls}`;
}
