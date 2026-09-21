// Обёртки над Tauri-командами backend'а.
import { invoke } from "@tauri-apps/api/core";

export interface FileSummary {
  source_name: string;
  total: number;
  category_count: Record<string, number>;
  /** Колонка, из которой взяты даты заявок. */
  date_column: string | null;
  /** Границы периода в формате 2026-07-01. */
  date_from: string | null;
  date_to: string | null;
  /** Сколько заявок имеют распознанную дату. */
  dated_rows: number;
}

export interface CategoryRule {
  name: string;
  include: string[];
  exclude: string[];
  weight: number;
  priority: number;
}

export interface CategoriesConfig {
  version: number;
  categories: CategoryRule[];
}

export const pickInputFiles = (): Promise<string[]> =>
  invoke("pick_input_files");

export const pickSavePath = (defaultName: string): Promise<string> =>
  invoke("pick_save_path", { defaultName });

export const classifyFiles = (paths: string[]): Promise<FileSummary[]> =>
  invoke("classify_files", { paths });

export const saveReportFile = (
  inputPath: string,
  outputPath: string,
  dateFrom: string,
  dateTo: string,
  groupBy: string,
): Promise<string> =>
  invoke("save_report_file", {
    inputPath,
    outputPath,
    dateFrom,
    dateTo,
    groupBy,
  });

export const getCategories = (): Promise<CategoriesConfig> =>
  invoke("get_categories");

export const setCategories = (configToml: string): Promise<void> =>
  invoke("set_categories", { configToml });

export const resetCategories = (): Promise<ResetOutcome> =>
  invoke("reset_categories");

export interface ResetOutcome {
  config: CategoriesConfig;
  /** "user" — пользовательский стандарт, "factory" — встроенный в программу */
  source: string;
}

export const setDefaultCategories = (configToml: string): Promise<void> =>
  invoke("set_default_categories", { configToml });

export interface DataPageResponse {
  file_name: string;
  headers: string[];
  rows: [string[], string][];
  total_filtered: number;
  total_file: number;
  page: number;
  page_size: number;
  total_pages: number;
  categories: string[];
  files: string[];
  /** Сколько строк отсеяно из-за отсутствия даты при заданном периоде. */
  rows_without_date: number;
  date_column: string | null;
}

export const getClassifiedPage = (
  fileIndex: number,
  page: number,
  pageSize: number,
  search: string,
  categoryFilter: string,
  dateFrom: string,
  dateTo: string,
): Promise<DataPageResponse> =>
  invoke("get_classified_page", {
    fileIndex,
    page,
    pageSize,
    search,
    categoryFilter,
    dateFrom,
    dateTo,
  });

export interface TimelinePoint {
  key: string;
  label: string;
  count: number;
}

export interface DatasetResponse {
  file_name: string;
  total_file: number;
  total_filtered: number;
  dated_rows: number;
  date_column: string | null;
  date_from: string | null;
  date_to: string | null;
  group_by: string;
  category_count: Record<string, number>;
  timeline: TimelinePoint[];
}

export const getDataset = (
  fileIndex: number,
  dateFrom: string,
  dateTo: string,
  groupBy: string,
): Promise<DatasetResponse> =>
  invoke("get_dataset", { fileIndex, dateFrom, dateTo, groupBy });
