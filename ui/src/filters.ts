// Общий фильтр по периоду: влияет на таблицу данных, графики и отчёт.
//
// Границы периода берутся из файла (колонка «Время создания»), отбор применяется
// на бэкенде: в выгрузке бывают десятки тысяч строк, целиком их в браузер не отдают.
import { getDataset, type DatasetResponse, type FileSummary } from "./api";

export type GroupBy = "month" | "week";

export interface PeriodState {
  /** Начало периода в формате 2026-07-01; пустая строка — без ограничения. */
  from: string;
  to: string;
  groupBy: GroupBy;
}

let state: PeriodState = { from: "", to: "", groupBy: "month" };
let fileIndex = 0;
let fileCount = 0;
let bounds: { from: string; to: string; dateColumn: string | null; datedRows: number } = {
  from: "",
  to: "",
  dateColumn: null,
  datedRows: 0,
};
let dataset: DatasetResponse | null = null;
const listeners: Array<() => void> = [];

export function getPeriod(): PeriodState {
  return { ...state };
}

export function getFileIndex(): number {
  return fileIndex;
}

/** Последний рассчитанный набор данных — из него рисуются графики. */
export function getDatasetSnapshot(): DatasetResponse | null {
  return dataset;
}

/** Подписка на изменение периода: таблица и графики перезагружаются сами. */
export function onPeriodChange(listener: () => void) {
  listeners.push(listener);
}

function emit() {
  for (const listener of listeners) listener();
}

/** Границы периода и список файлов, полученные после классификации. */
export function setBounds(summaries: FileSummary[]) {
  fileCount = summaries.length;
  const withDates = summaries.filter((s) => s.date_from && s.date_to);
  if (withDates.length > 0) {
    const from = withDates.map((s) => s.date_from!).sort()[0];
    const to = withDates.map((s) => s.date_to!).sort().slice(-1)[0];
    bounds = {
      from,
      to,
      dateColumn: withDates[0].date_column,
      datedRows: summaries.reduce((sum, s) => sum + s.dated_rows, 0),
    };
  } else {
    bounds = { from: "", to: "", dateColumn: null, datedRows: 0 };
  }

  state = { ...state, from: "", to: "" };
  fileIndex = 0;
  syncControls();
}

export function initFilters() {
  const fromInput = document.getElementById("date-from") as HTMLInputElement;
  const toInput = document.getElementById("date-to") as HTMLInputElement;
  const groupSelect = document.getElementById("group-by") as HTMLSelectElement;
  const fileSelect = document.getElementById("filter-file") as HTMLSelectElement;

  fromInput.addEventListener("change", () => {
    state.from = fromInput.value;
    applyChange();
  });
  toInput.addEventListener("change", () => {
    state.to = toInput.value;
    applyChange();
  });
  groupSelect.addEventListener("change", () => {
    state.groupBy = groupSelect.value === "week" ? "week" : "month";
    applyChange();
  });
  fileSelect.addEventListener("change", () => {
    fileIndex = parseInt(fileSelect.value, 10) || 0;
    applyChange();
  });

  for (const button of Array.from(document.querySelectorAll<HTMLButtonElement>("[data-preset]"))) {
    button.addEventListener("click", () => applyPreset(button.dataset.preset || "all"));
  }
}

function applyChange() {
  syncControls();
  void refreshDataset();
}

async function applyPreset(preset: string) {
  const anchor = bounds.to || bounds.from;
  switch (preset) {
    case "last-month":
      if (anchor) {
        state.from = monthStart(anchor);
        state.to = monthEnd(anchor);
      }
      break;
    case "prev-month":
      if (anchor) {
        const start = monthStart(anchor);
        state.from = monthStart(addDays(start, -1));
        state.to = monthEnd(addDays(start, -1));
      }
      break;
    case "last-7":
      if (anchor) {
        state.from = addDays(anchor, -6);
        state.to = anchor;
      }
      break;
    case "last-30":
      if (anchor) {
        state.from = addDays(anchor, -29);
        state.to = anchor;
      }
      break;
    default:
      state.from = "";
      state.to = "";
  }
  applyChange();
}

/** Пересчитывает набор данных для графиков и уведомляет подписчиков. */
export async function refreshDataset() {
  try {
    dataset = await getDataset(fileIndex, state.from, state.to, state.groupBy);
  } catch (error) {
    dataset = null;
    const info = document.getElementById("filter-info");
    if (info) info.textContent = `Не удалось получить данные: ${error}`;
    return;
  }
  renderInfo();
  emit();
}

function renderInfo() {
  const info = document.getElementById("filter-info");
  if (!info) return;

  if (!dataset || dataset.total_file === 0) {
    info.textContent = "Файлы ещё не классифицированы";
    return;
  }
  if (!bounds.dateColumn) {
    info.textContent = "Колонка с датой не найдена — отбор по периоду недоступен";
    return;
  }

  const parts = [
    `Отобрано ${dataset.total_filtered} из ${dataset.total_file}`,
    `колонка «${bounds.dateColumn}»`,
  ];
  if (dataset.dated_rows < dataset.total_file) {
    parts.push(`без даты: ${dataset.total_file - dataset.dated_rows}`);
  }
  if (state.from || state.to) {
    parts.push(
      `${state.from || "начало"} — ${state.to || "конец"}`,
    );
  } else if (bounds.from && bounds.to) {
    parts.push(`доступно ${bounds.from} — ${bounds.to}`);
  }
  info.textContent = parts.join(" · ");
}

/** Приводит элементы управления в соответствие с состоянием. */
function syncControls() {
  const bar = document.getElementById("filter-bar");
  const fromInput = document.getElementById("date-from") as HTMLInputElement | null;
  const toInput = document.getElementById("date-to") as HTMLInputElement | null;
  const fileSelect = document.getElementById("filter-file") as HTMLSelectElement | null;
  const groupSelect = document.getElementById("group-by") as HTMLSelectElement | null;

  if (bar) bar.style.display = fileCount > 0 ? "" : "none";
  if (fromInput) {
    fromInput.min = bounds.from;
    fromInput.max = bounds.to;
    fromInput.value = state.from;
  }
  if (toInput) {
    toInput.min = bounds.from;
    toInput.max = bounds.to;
    toInput.value = state.to;
  }
  if (groupSelect) groupSelect.value = state.groupBy;
  if (fileSelect) {
    fileSelect.style.display = fileCount > 1 ? "" : "none";
  }
  // Без колонки с датой поля периода бессмысленны
  const disabled = !bounds.dateColumn;
  for (const element of [fromInput, toInput, ...Array.from(document.querySelectorAll<HTMLButtonElement>("[data-preset]"))]) {
    if (element) element.disabled = disabled;
  }
}

/** Имена файлов для выпадающего списка в панели фильтра. */
export function setFileOptions(names: string[]) {
  const select = document.getElementById("filter-file") as HTMLSelectElement | null;
  if (!select) return;
  select.innerHTML = "";
  names.forEach((name, index) => {
    const option = document.createElement("option");
    option.value = String(index);
    option.textContent = name;
    select.appendChild(option);
  });
  select.value = String(fileIndex);
}

// --- Арифметика дат в UTC, чтобы не зависеть от часового пояса ---

function parse(value: string): Date {
  return new Date(`${value}T00:00:00Z`);
}

function iso(date: Date): string {
  return date.toISOString().slice(0, 10);
}

export function addDays(value: string, days: number): string {
  const date = parse(value);
  date.setUTCDate(date.getUTCDate() + days);
  return iso(date);
}

export function monthStart(value: string): string {
  const date = parse(value);
  return iso(new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), 1)));
}

export function monthEnd(value: string): string {
  const date = parse(value);
  return iso(new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1, 0)));
}
