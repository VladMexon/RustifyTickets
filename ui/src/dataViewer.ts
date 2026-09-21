// Вкладка просмотра классифицированных данных с пагинацией и поиском.
import { getClassifiedPage, type DataPageResponse } from "./api";
import { getFileIndex, getPeriod, onPeriodChange } from "./filters";

let currentPage = 1;
let currentPageSize = 50;
let currentSearch = "";
let currentCategoryFilter = "";
let debounceTimer: number | null = null;
// Какие столбцы скрыты пользователем (по имени заголовка)
let hiddenColumns = new Set<string>();
// Заголовки текущего файла — для панели фильтра столбцов
let currentHeaders: string[] = [];

export function initDataViewer() {
  // Смена периода или файла в панели фильтра перезагружает таблицу
  onPeriodChange(() => {
    currentPage = 1;
    void loadData();
  });

  const searchInput = document.getElementById("data-search") as HTMLInputElement;
  const catFilter = document.getElementById("data-cat-filter") as HTMLSelectElement;
  const pageSizeSelect = document.getElementById("data-page-size") as HTMLSelectElement;
  const colFilterBtn = document.getElementById("data-col-filter-btn") as HTMLButtonElement;
  const colFilterPanel = document.getElementById("data-col-filter-panel") as HTMLDivElement;

  const firstBtn = document.getElementById("data-first-page") as HTMLButtonElement;
  const prevBtn = document.getElementById("data-prev-page") as HTMLButtonElement;
  const nextBtn = document.getElementById("data-next-page") as HTMLButtonElement;
  const lastBtn = document.getElementById("data-last-page") as HTMLButtonElement;

  // --- Фильтр столбцов ---
  colFilterBtn?.addEventListener("click", (e) => {
    e.stopPropagation();
    const isOpen = colFilterPanel.style.display !== "none";
    colFilterPanel.style.display = isOpen ? "none" : "";
    if (!isOpen) renderColumnFilterPanel(colFilterPanel);
  });
  // Закрытие панели при клике вне неё
  document.addEventListener("click", (e) => {
    if (!colFilterPanel || colFilterPanel.style.display === "none") return;
    const t = e.target as HTMLElement;
    if (!t.closest(".col-filter-wrap")) {
      colFilterPanel.style.display = "none";
    }
  });

  searchInput?.addEventListener("input", () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = window.setTimeout(() => {
      currentSearch = searchInput.value;
      currentPage = 1;
      loadData();
    }, 250);
  });

  catFilter?.addEventListener("change", () => {
    currentCategoryFilter = catFilter.value;
    currentPage = 1;
    loadData();
  });

  pageSizeSelect?.addEventListener("change", () => {
    currentPageSize = parseInt(pageSizeSelect.value, 10) || 50;
    currentPage = 1;
    loadData();
  });

  firstBtn?.addEventListener("click", () => {
    currentPage = 1;
    loadData();
  });

  prevBtn?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      loadData();
    }
  });

  nextBtn?.addEventListener("click", () => {
    currentPage++;
    loadData();
  });

  lastBtn?.addEventListener("click", () => {
    currentPage = 999999;
    loadData();
  });
}

export async function refreshDataViewer() {
  currentPage = 1;
  await loadData();
}

function renderColumnFilterPanel(panel: HTMLDivElement) {
  panel.innerHTML = "";

  const title = document.createElement("div");
  title.className = "col-filter-title";
  title.textContent = "Отображаемые столбцы";
  panel.appendChild(title);

  const actions = document.createElement("div");
  actions.className = "col-filter-actions";
  const allBtn = document.createElement("button");
  allBtn.textContent = "Все";
  allBtn.addEventListener("click", () => {
    hiddenColumns.clear();
    renderColumnFilterPanel(panel);
    loadData();
  });
  const noneBtn = document.createElement("button");
  noneBtn.textContent = "Только категория";
  noneBtn.addEventListener("click", () => {
    hiddenColumns = new Set(currentHeaders);
    renderColumnFilterPanel(panel);
    loadData();
  });
  actions.append(allBtn, noneBtn);
  panel.appendChild(actions);

  for (const h of currentHeaders) {
    const label = document.createElement("label");
    label.className = "col-filter-item";
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = !hiddenColumns.has(h);
    cb.addEventListener("change", () => {
      if (cb.checked) hiddenColumns.delete(h);
      else hiddenColumns.add(h);
      loadData();
    });
    const span = document.createElement("span");
    span.textContent = h;
    label.append(cb, span);
    panel.appendChild(label);
  }
}

async function loadData() {
  const container = document.getElementById("data-table-container");
  const countInfo = document.getElementById("data-count-info");
  const pageIndicator = document.getElementById("data-page-indicator");
  const catFilter = document.getElementById("data-cat-filter") as HTMLSelectElement;

  const firstBtn = document.getElementById("data-first-page") as HTMLButtonElement;
  const prevBtn = document.getElementById("data-prev-page") as HTMLButtonElement;
  const nextBtn = document.getElementById("data-next-page") as HTMLButtonElement;
  const lastBtn = document.getElementById("data-last-page") as HTMLButtonElement;

  if (!container) return;

  try {
    const period = getPeriod();
    const res: DataPageResponse = await getClassifiedPage(
      getFileIndex(),
      currentPage,
      currentPageSize,
      currentSearch,
      currentCategoryFilter,
      period.from,
      period.to,
    );

    // Метка на кнопке фильтра столбцов
    const colBtn = document.getElementById("data-col-filter-btn") as HTMLButtonElement | null;
    if (colBtn) {
      const hiddenCount = res.headers.filter((h) => hiddenColumns.has(h)).length;
      colBtn.textContent = hiddenCount > 0 ? `Столбцы (${hiddenCount} скрыто) ▾` : "Столбцы ▾";
    }

    // Обновляем список категорий для фильтра
    if (catFilter) {
      const selectedVal = catFilter.value;
      const existingCats = Array.from(catFilter.options)
        .slice(1)
        .map((o) => o.value);
      if (JSON.stringify(existingCats) !== JSON.stringify(res.categories)) {
        catFilter.innerHTML = '<option value="">Все категории</option>';
        res.categories.forEach((cat) => {
          const opt = document.createElement("option");
          opt.value = cat;
          opt.textContent = cat;
          if (cat === selectedVal) opt.selected = true;
          catFilter.appendChild(opt);
        });
      }
    }

    if (res.total_file === 0) {
      container.innerHTML =
        '<div class="empty-placeholder">Файлы ещё не классифицированы. Перейдите на вкладку «Файлы и отчёт» и нажмите «Классифицировать».</div>';
      if (countInfo) countInfo.textContent = "Нет данных для отображения";
      if (pageIndicator) pageIndicator.textContent = "Стр. 0 из 0";
      if (firstBtn) firstBtn.disabled = true;
      if (prevBtn) prevBtn.disabled = true;
      if (nextBtn) nextBtn.disabled = true;
      if (lastBtn) lastBtn.disabled = true;
      return;
    }

    currentPage = res.page;

    if (countInfo) {
      const start = res.total_filtered > 0 ? (res.page - 1) * res.page_size + 1 : 0;
      const end = Math.min(res.page * res.page_size, res.total_filtered);
      const parts = [
        `Строки ${start}–${end} из ${res.total_filtered}`,
        `всего в файле: ${res.total_file}`,
      ];
      if ((period.from || period.to) && res.rows_without_date > 0) {
        parts.push(`без даты: ${res.rows_without_date}`);
      }
      countInfo.textContent = parts.join(" · ");
    }

    if (pageIndicator) {
      pageIndicator.textContent = `Стр. ${res.page} из ${res.total_pages}`;
    }

    if (firstBtn) firstBtn.disabled = res.page <= 1;
    if (prevBtn) prevBtn.disabled = res.page <= 1;
    if (nextBtn) nextBtn.disabled = res.page >= res.total_pages;
    if (lastBtn) lastBtn.disabled = res.page >= res.total_pages;

    if (res.rows.length === 0) {
      container.innerHTML =
        '<div class="empty-placeholder">По заданным условиям поиска ничего не найдено</div>';
      return;
    }

    // Рендерим таблицу
    const table = document.createElement("table");
    table.className = "data-table";

    currentHeaders = res.headers;

    // Индексы отображаемых столбцов (скрытые исключаем)
    const visibleCols = res.headers
      .map((h, i) => ({ header: h, index: i }))
      .filter(({ header }) => !hiddenColumns.has(header));

    // Заголовки
    const thead = table.createTHead();
    const hRow = thead.insertRow();

    // Номер строки
    const thIdx = document.createElement("th");
    thIdx.textContent = "#";
    thIdx.className = "col-index";
    hRow.appendChild(thIdx);

    visibleCols.forEach(({ header }) => {
      const th = document.createElement("th");
      th.textContent = header;
      hRow.appendChild(th);
    });

    const thCat = document.createElement("th");
    thCat.textContent = "Категория";
    thCat.className = "col-cat";
    hRow.appendChild(thCat);

    // Данные
    const tbody = table.createTBody();
    const baseIndex = (res.page - 1) * res.page_size;

    res.rows.forEach(([cells, category], rowIdx) => {
      const tr = tbody.insertRow();

      const tdIdx = tr.insertCell();
      tdIdx.className = "col-index";
      tdIdx.textContent = String(baseIndex + rowIdx + 1);

      visibleCols.forEach(({ index }) => {
        const val = cells[index] ?? "";
        const td = tr.insertCell();
        td.textContent = val;
        td.title = val;
      });

      const tdCat = tr.insertCell();
      tdCat.className = "col-cat";
      const badge = document.createElement("span");
      badge.textContent = category;
      if (category === "Не классифицировано") {
        badge.className = "badge badge-unclassified";
      } else {
        badge.className = "badge badge-category";
      }
      tdCat.appendChild(badge);
    });

    container.innerHTML = "";
    container.appendChild(table);
  } catch (err) {
    container.innerHTML = `<div class="status-line error">Ошибка загрузки данных: ${err}</div>`;
  }
}
