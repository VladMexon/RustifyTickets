// Графики на базе Chart.js: сводка по всем загруженным файлам.
import Chart from "chart.js/auto";
import type { FileSummary } from "./api";

const PALETTE = [
  "#2e69ff", "#1f9d55", "#d64545", "#e6a817", "#8a4fd3",
  "#17b2c3", "#e2703a", "#5a6b7b", "#c74f9a", "#67ab2f",
  "#3944bc", "#b5a615", "#6d3fb0", "#0f8a8a", "#a5361f",
];

let doughnutChart: Chart | null = null;
let barChart: Chart | null = null;

export function renderCharts(summaries: FileSummary[]) {
  if (summaries.length === 0) return;

  // Агрегируем по категориям через все файлы
  const totals = new Map<string, number>();
  let grandTotal = 0;
  for (const s of summaries) {
    for (const [cat, count] of Object.entries(s.category_count)) {
      totals.set(cat, (totals.get(cat) || 0) + count);
      grandTotal += count;
    }
  }

  // Сортировка по убыванию
  const sorted = [...totals.entries()].sort((a, b) => b[1] - a[1]);
  const labels = sorted.map(([cat]) => cat);
  const values = sorted.map(([, v]) => v);
  const colors = labels.map((_, i) => PALETTE[i % PALETTE.length]);

  renderSummaryTable(summaries, sorted, grandTotal);

  // --- Кольцевая: доли категорий ---
  const doughnutCtx = (
    document.getElementById("chart-doughnut") as HTMLCanvasElement
  ).getContext("2d")!;
  doughnutChart?.destroy();
  doughnutChart = new Chart(doughnutCtx, {
    type: "doughnut",
    data: { labels, datasets: [{ data: values, backgroundColor: colors }] },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        title: { display: true, text: "Распределение заявок по категориям" },
        legend: { position: "right" },
      },
    },
  });

  // --- Гистограмма: топ-10 ---
  const top = sorted.slice(0, 10);
  const barCtx = (
    document.getElementById("chart-bar") as HTMLCanvasElement
  ).getContext("2d")!;
  barChart?.destroy();
  barChart = new Chart(barCtx, {
    type: "bar",
    data: {
      labels: top.map(([cat]) => cat),
      datasets: [
        {
          label: "Заявок",
          data: top.map(([, v]) => v),
          backgroundColor: colors.slice(0, top.length),
        },
      ],
    },
    options: {
      indexAxis: "y",
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        title: { display: true, text: "Топ-10 категорий по количеству заявок" },
        legend: { display: false },
      },
      scales: { x: { beginAtZero: true } },
    },
  });
}

function renderSummaryTable(
  summaries: FileSummary[],
  sorted: [string, number][],
  grandTotal: number,
) {
  const wrap = document.getElementById("summary-table-wrap")!;
  wrap.innerHTML = "";
  const table = document.createElement("table");
  table.className = "summary-table";

  const isMultiFile = summaries.length > 1;

  // Заголовок таблицы
  const header = table.insertRow();
  const thCat = document.createElement("th");
  thCat.textContent = "Категория";
  header.appendChild(thCat);

  if (isMultiFile) {
    for (const s of summaries) {
      const th = document.createElement("th");
      th.className = "num";
      th.textContent = s.source_name;
      th.title = `${s.source_name} (всего заявок: ${s.total})`;
      header.appendChild(th);
    }
  }

  const thTotal = document.createElement("th");
  thTotal.className = "num";
  thTotal.textContent = isMultiFile ? "Итого" : "Количество";
  header.appendChild(thTotal);

  const thPct = document.createElement("th");
  thPct.className = "num";
  thPct.textContent = "Доля";
  header.appendChild(thPct);

  // Строки категорий
  for (const [cat, count] of sorted) {
    const row = table.insertRow();

    // Название категории
    const catCell = row.insertCell();
    catCell.textContent = cat;
    if (cat === "Не классифицировано") {
      catCell.className = "unclassified-cell";
    }

    // Если несколько файлов - выводим разбивку по каждому файлу
    if (isMultiFile) {
      for (const s of summaries) {
        const fileCell = row.insertCell();
        fileCell.className = "num";
        const val = s.category_count[cat] || 0;
        fileCell.textContent = val > 0 ? String(val) : "0";
      }
    }

    // Итоговое количество по категории
    const countCell = row.insertCell();
    countCell.className = "num font-semibold";
    countCell.textContent = String(count);

    // Доля в процентах
    const pct = row.insertCell();
    pct.className = "num";
    const percent = grandTotal > 0 ? (count / grandTotal) * 100 : 0;
    pct.textContent = `${percent.toFixed(1)}%`;
  }

  // Итоговая строка
  const totalRow = table.insertRow();
  totalRow.className = "summary-total-row";

  const totalCat = totalRow.insertCell();
  totalCat.textContent = "Всего заявок";

  if (isMultiFile) {
    for (const s of summaries) {
      const fileTotal = totalRow.insertCell();
      fileTotal.className = "num font-semibold";
      fileTotal.textContent = String(s.total);
    }
  }

  const grandCell = totalRow.insertCell();
  grandCell.className = "num font-semibold";
  grandCell.textContent = String(grandTotal);

  const pctTotal = totalRow.insertCell();
  pctTotal.className = "num font-semibold";
  pctTotal.textContent = "100.0%";

  wrap.appendChild(table);
}
