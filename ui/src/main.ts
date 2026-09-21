// Точка входа GUI: вкладки, инициализация панелей.
import { initUploadPanel } from "./upload";
import { initEditor } from "./editor";
import { initDataViewer, refreshDataViewer } from "./dataViewer";
import {
  getDatasetSnapshot,
  initFilters,
  onPeriodChange,
} from "./filters";

import "./style.css";

const app = document.getElementById("app")!;
app.innerHTML = `
  <h1>RustifyTickets</h1>
  <p class="subtitle">Классификация заявок-service desk по ключевым словам</p>

  <div class="tabs">
    <button class="tab active" data-tab="upload">Файлы и отчёт</button>
    <button class="tab" data-tab="data">Данные</button>
    <button class="tab" data-tab="charts">Графики</button>
    <button class="tab" data-tab="editor">Категории</button>
  </div>

  <div class="filter-bar card" id="filter-bar" style="display:none">
    <span class="filter-label">Период по «Время создания»:</span>
    <input type="date" id="date-from" title="Начало периода" />
    <span class="filter-sep">—</span>
    <input type="date" id="date-to" title="Конец периода" />
    <div class="filter-presets">
      <button data-preset="all">Весь период</button>
      <button data-preset="last-month">Последний месяц</button>
      <button data-preset="prev-month">Предыдущий месяц</button>
      <button data-preset="last-7">7 дней</button>
      <button data-preset="last-30">30 дней</button>
    </div>
    <label class="filter-group">Группировка
      <select id="group-by">
        <option value="month">по месяцам</option>
        <option value="week">по неделям</option>
      </select>
    </label>
    <select id="filter-file" title="Файл" style="display:none"></select>
    <span class="filter-info" id="filter-info"></span>
  </div>

  <div class="panel active" id="panel-upload">
    <div class="card">
      <h2>Загрузка файлов</h2>
      <div class="dropzone" id="dropzone">
        Перетащите xlsx-файлы сюда или
        <button class="primary" id="pick-files">Выберите файлы</button>
      </div>
      <ul class="file-list" id="file-list"></ul>
      <div class="progress-wrap">
        <div class="progress-bar"><div id="progress-fill" style="width:0%"></div></div>
        <span class="progress-label" id="progress-label"></span>
      </div>
      <div style="margin-top:12px; display:flex; gap:10px;">
        <button class="primary" id="classify-btn" disabled>Классифицировать</button>
        <button id="save-report-btn" disabled>Сохранить отчёт (Excel)</button>
      </div>
      <div class="status-line" id="upload-status"></div>
    </div>
  </div>

  <div class="panel" id="panel-data">
    <div class="card">
      <h2>Классифицированные данные</h2>
      <div class="data-toolbar">
        <input class="search-box inline" id="data-search" placeholder="Поиск по тексту заявки..." />
        <select id="data-cat-filter">
          <option value="">Все категории</option>
        </select>
        <div class="col-filter-wrap">
          <button id="data-col-filter-btn" title="Показать/скрыть столбцы">Столбцы ▾</button>
          <div class="col-filter-panel" id="data-col-filter-panel" style="display:none"></div>
        </div>
        <select id="data-page-size" title="Строк на странице">
          <option value="20">20 строк</option>
          <option value="50" selected>50 строк</option>
          <option value="100">100 строк</option>
          <option value="200">200 строк</option>
        </select>
      </div>
      <div class="data-meta">
        <span id="data-count-info"></span>
        <span class="data-pagination">
          <button id="data-first-page" title="Первая страница" disabled>&laquo;</button>
          <button id="data-prev-page" title="Предыдущая" disabled>&lsaquo;</button>
          <span id="data-page-indicator">Стр. 0 из 0</span>
          <button id="data-next-page" title="Следующая" disabled>&rsaquo;</button>
          <button id="data-last-page" title="Последняя" disabled>&raquo;</button>
        </span>
      </div>
      <div class="data-table-container" id="data-table-container">
        <div class="empty-placeholder">Файлы ещё не классифицированы. Перейдите на вкладку «Файлы и отчёт» и нажмите «Классифицировать».</div>
      </div>
    </div>
  </div>

  <div class="panel" id="panel-charts">
    <div class="card">
      <h2>Графики за выбранный период</h2>
      <div class="empty-placeholder" id="charts-empty">
        Файлы ещё не классифицированы. Перейдите на вкладку «Файлы и отчёт» и нажмите «Классифицировать».
      </div>
      <div class="charts-grid" id="charts-grid" style="display:none">
        <div class="chart-box"><canvas id="chart-doughnut"></canvas></div>
        <div class="chart-box"><canvas id="chart-bar"></canvas></div>
        <div class="chart-box wide"><canvas id="chart-timeline"></canvas></div>
        <div class="chart-box wide" id="summary-table-wrap"></div>
      </div>
    </div>
  </div>

  <div class="panel" id="panel-editor">
    <div class="card">
      <h2>Списки ключевых слов</h2>
      <div class="editor-toolbar">
        <button class="primary" id="save-cats">Сохранить</button>
        <button id="make-default-cats" title="Запомнить текущие правила как эталон для сброса">Сделать стандартными</button>
        <button id="reset-cats">Сбросить к стандартным</button>
        <button id="add-cat">+ Категория</button>
        <button id="cat-expand-all">Развернуть все</button>
        <button id="cat-collapse-all">Свернуть все</button>
        <span class="editor-hint" id="cat-count"></span>
      </div>
      <p class="editor-note">
        Заявка относится к категории с наибольшей <b>суммой весов</b> совпавших слов
        (вес по умолчанию — 1 за слово). При равенстве весов побеждает категория
        с меньшим <b>приоритетом</b>. Сопоставление — по подстроке; пишите корень
        слова без окончания («газоанализ» покроет «газоанализатор» и
        «газоанализаторы»). <b>Регистр и вид дефиса не важны</b>: слово «Wi-Fi»
        найдёт «wi-fi», «wi–fi» и «wi fi», а английские слова работают наравне с
        русскими. Слитное написание — отдельное слово: «wifi» не совпадёт с «wi-fi».
        Слова-исключения запрещают категорию для заявки.
        Кнопка <b>«Сделать стандартными»</b> запоминает текущий набор правил как
        эталон: именно его вернёт <b>«Сбросить к стандартным»</b>.
      </p>
      <input class="search-box" id="cat-search" placeholder="Поиск по категориям и словам..." />
      <div class="category-list" id="category-list"></div>
      <div class="status-line" id="editor-status"></div>
    </div>
  </div>
`;

// --- Вкладки ---
for (const tab of Array.from(document.querySelectorAll(".tab"))) {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
    document.querySelectorAll(".panel").forEach((p) => p.classList.remove("active"));
    tab.classList.add("active");
    const panel = document.getElementById(`panel-${(tab as HTMLElement).dataset.tab}`);
    panel?.classList.add("active");
    // Таблица перезагружается при входе на вкладку: нужны свежие данные
    if ((tab as HTMLElement).dataset.tab === "data") {
      void refreshDataViewer();
    }
  });
}

initUploadPanel(async (dataset) => {
  const { renderCharts } = await import("./charts");
  renderCharts(dataset);
});
initEditor();
initDataViewer();

// Панель периода общая для таблицы, графиков и отчёта
initFilters();
onPeriodChange(async () => {
  const { renderCharts } = await import("./charts");
  renderCharts(getDatasetSnapshot());
});
