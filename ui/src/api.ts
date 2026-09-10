// Обёртки над Tauri-командами backend'а.
import { invoke } from "@tauri-apps/api/core";

export interface FileSummary {
  source_name: string;
  total: number;
  category_count: Record<string, number>;
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
): Promise<string> => invoke("save_report_file", { inputPath, outputPath });

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
}

export const getClassifiedPage = (
  fileIndex: number,
  page: number,
  pageSize: number,
  search: string,
  categoryFilter: string,
): Promise<DataPageResponse> =>
  invoke("get_classified_page", {
    fileIndex,
    page,
    pageSize,
    search,
    categoryFilter,
  });
