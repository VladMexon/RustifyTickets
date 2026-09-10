// Панель загрузки и классификации файлов.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  classifyFiles,
  saveReportFile,
  type FileSummary,
} from "./api";

export interface FileChosen {
  path: string;
  name: string;
}

let files: FileChosen[] = [];
let lastSummaries: FileSummary[] = [];
let unlistenProgress: (() => void) | null = null;
let unlistenDragDrop: (() => void) | null = null;

const listEl = () => document.getElementById("file-list") as HTMLUListElement;
const statusEl = () => document.getElementById("upload-status") as HTMLDivElement;

export function initUploadPanel(renderCharts: (s: FileSummary[]) => void) {
  const pickBtn = document.getElementById("pick-files") as HTMLButtonElement;
  const classifyBtn = document.getElementById("classify-btn") as HTMLButtonElement;
  const saveBtn = document.getElementById("save-report-btn") as HTMLButtonElement;
  const dropzone = document.getElementById("dropzone") as HTMLDivElement;

  const openDialog = async () => {
    try {
      const picked = await open({
        multiple: true,
        filters: [{ name: "Excel", extensions: ["xlsx", "xlsm", "xls"] }],
      });
      if (!picked) return;
      const paths = Array.isArray(picked) ? picked : [picked];
      for (const p of paths) addFile(p);
      render();
    } catch (e) {
      setStatus(String(e), "error");
    }
  };

  pickBtn.addEventListener("click", openDialog);
  // Клик по всей зоне тоже открывает выбор файлов (кроме клика по кнопке)
  dropzone.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).closest("button")) return;
    openDialog();
  });

  dropzone.addEventListener("dragover", (e) => {
    e.preventDefault();
    dropzone.classList.add("dragover");
  });
  dropzone.addEventListener("dragleave", () => dropzone.classList.remove("dragover"));

  // Tauri перехватывает drag-and-drop нативно: HTML5-события не отдают
  // пути файлов. Подписываемся на собственное событие webview.
  // Типы событий: enter/over (подсветка), drop (файлы), leave.
  getCurrentWebview()
    .onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "enter" || payload.type === "over") {
        dropzone.classList.add("dragover");
      }
      if (payload.type === "drop") {
        dropzone.classList.remove("dragover");
        for (const p of payload.paths) addFile(p);
        render();
      }
      if (payload.type === "leave") {
        dropzone.classList.remove("dragover");
      }
    })
    .then((un) => (unlistenDragDrop = un));

  classifyBtn.addEventListener("click", async () => {
    if (files.length === 0) return;
    classifyBtn.disabled = true;
    setStatus("Обработка...", "");
    try {
      lastSummaries = await classifyFiles(files.map((f) => f.path));
      setStatus(`Обработано файлов: ${lastSummaries.length}`, "ok");
      renderCharts(lastSummaries);
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      classifyBtn.disabled = false;
    }
  });

  saveBtn.addEventListener("click", async () => {
    if (files.length === 0) return;
    for (const f of files) {
      const base = f.name.replace(/\.(xlsx|xlsm|xls)$/i, "");
      const target = await save({
        defaultPath: `${base}_classified.xlsx`,
        filters: [{ name: "Excel", extensions: ["xlsx"] }],
      });
      if (!target) continue;
      try {
        await saveReportFile(f.path, target);
        setStatus(`Отчёт сохранён: ${target}`, "ok");
      } catch (e) {
        setStatus(String(e), "error");
        return;
      }
    }
  });

  // Прогресс из backend'а
  listen<{ processed: number; total: number; current_file: string }>(
    "classify-progress",
    (event) => {
      const { processed, total, current_file } = event.payload;
      const bar = document.getElementById("progress-fill") as HTMLDivElement;
      const label = document.getElementById("progress-label") as HTMLSpanElement;
      bar.style.width = total ? `${(processed / total) * 100}%` : "0%";
      label.textContent = `(${processed}/${total}) ${current_file}`;
    },
  ).then((un) => (unlistenProgress = un));
}

function addFile(path: string) {
  if (files.some((f) => f.path === path)) return;
  files.push({ path, name: path.split(/[\\/]/).pop() || path });
}

function render() {
  const list = listEl();
  list.innerHTML = "";
  for (const f of files) {
    const li = document.createElement("li");
    const name = document.createElement("span");
    name.textContent = f.name;
    const remove = document.createElement("button");
    remove.className = "remove";
    remove.textContent = "✕";
    remove.title = "Убрать файл";
    remove.addEventListener("click", () => {
      files = files.filter((x) => x.path !== f.path);
      render();
    });
    li.append(name, remove);
    list.appendChild(li);
  }
  (document.getElementById("classify-btn") as HTMLButtonElement).disabled = files.length === 0;
  (document.getElementById("save-report-btn") as HTMLButtonElement).disabled = files.length === 0;
}

function setStatus(text: string, cls: string) {
  const el = statusEl();
  el.textContent = text;
  el.className = `status-line ${cls}`;
}

export function getLastSummaries(): FileSummary[] {
  return lastSummaries;
}
