// Графики: распределение по категориям и динамика по месяцам или неделям.
import Chart from "chart.js/auto";
import type { DatasetResponse } from "./api";
import { getPeriod } from "./filters";

const PALETTE = [
  "#2e69ff", "#1f9d55", "#d64545", "#e6a817", "#8a4fd3",
  "#17b2c3", "#e2703a", "#5a6b7b", "#c74f9a", "#67ab2f",
  "#3944bc", "#b5a615", "#6d3fb0", "#0f8a8a", "#a5361f",
];

let doughnutChart: Chart | null = null;
let barChart: Chart | null = null;
let timelineChart: Chart | null = null;

/** Перерисовывает вкладку «Графики» по текущему отбору. */
export function renderCharts(dataset: DatasetResponse | null) {
  const empty = document.getElementById("charts-empty") as HTMLDivElement | null;
  const grid = document.getElementById("charts-grid") as HTMLDivElement | null;

  if (!dataset || dataset.total_file === 0) {
    if (empty) empty.style.display = "";
    if (grid) grid.style.display = "none";
    return;
  }
  if (empty) empty.style.display = "none";
  if (grid) grid.style.display = "";

  const sorted = Object.entries(dataset.category_count).sort((a, b) => b[1] - a[1]);
  const labels = sorted.map(([name]) => name);
  const values = sorted.map(([, count]) => count);
  const colors = labels.map((_, index) => PALETTE[index % PALETTE.length]);

  renderSummaryTable(sorted, dataset);

  // --- Кольцевая: доли категорий в отобранном периоде ---
  const doughnutCanvas = document.getElementById("chart-doughnut") as HTMLCanvasElement;
  doughnutChart?.destroy();
  doughnutChart = new Chart(doughnutCanvas.getContext("2d")!, {
    type: "doughnut",
    data: { labels, datasets: [{ data: values, backgroundColor: colors }] },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        title: {
          display: true,
          text: `Распределение заявок по категориям (${dataset.total_filtered})`,
        },
        legend: { position: "right" },
      },
    },
  });

  // --- Гистограмма: топ-10 ---
  const top = sorted.slice(0, 10);
  const barCanvas = document.getElementById("chart-bar") as HTMLCanvasElement;
  barChart?.destroy();
  barChart = new Chart(barCanvas.getContext("2d")!, {
    type: "bar",
    data: {
      labels: top.map(([name]) => name),
      datasets: [
        {
          label: "Заявок",
          data: top.map(([, count]) => count),
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

  // --- Динамика по периодам ---
  const { groupBy } = getPeriod();
  const timelineCanvas = document.getElementById("chart-timeline") as HTMLCanvasElement;
  timelineChart?.destroy();
  timelineChart = new Chart(timelineCanvas.getContext("2d")!, {
    type: groupBy === "week" ? "bar" : "line",
    data: {
      labels: dataset.timeline.map((point) => point.label),
      datasets: [
        {
          label: "Заявок за период",
          data: dataset.timeline.map((point) => point.count),
          backgroundColor: "#2e69ff",
          borderColor: "#2e69ff",
          borderWidth: 2,
          fill: false,
          tension: 0.25,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        title: {
          display: true,
          text:
            groupBy === "week"
              ? "Динамика заявок по неделям"
              : "Динамика заявок по месяцам",
        },
        legend: { display: false },
      },
      scales: { y: { beginAtZero: true } },
    },
  });
}

function renderSummaryTable(sorted: [string, number][], dataset: DatasetResponse) {
  const wrap = document.getElementById("summary-table-wrap");
  if (!wrap) return;
  wrap.innerHTML = "";

  const table = document.createElement("table");
  table.className = "summary-table";

  const header = table.insertRow();
  for (const [index, text] of ["Категория", "Количество", "Доля"].entries()) {
    const th = document.createElement("th");
    th.textContent = text;
    if (index > 0) th.className = "num";
    header.appendChild(th);
  }

  const total = dataset.total_filtered || 1;
  for (const [name, count] of sorted) {
    const row = table.insertRow();

    const nameCell = row.insertCell();
    nameCell.textContent = name;
    if (name === "Не классифицировано") nameCell.className = "unclassified-cell";

    const countCell = row.insertCell();
    countCell.className = "num font-semibold";
    countCell.textContent = String(count);

    const pctCell = row.insertCell();
    pctCell.className = "num";
    pctCell.textContent = `${((count / total) * 100).toFixed(1)}%`;
  }

  const totalRow = table.insertRow();
  totalRow.className = "summary-total-row";
  const label = totalRow.insertCell();
  label.textContent = "Всего за период";
  const sum = totalRow.insertCell();
  sum.className = "num font-semibold";
  sum.textContent = String(dataset.total_filtered);
  const pct = totalRow.insertCell();
  pct.className = "num font-semibold";
  pct.textContent = "100.0%";

  wrap.appendChild(table);
}
