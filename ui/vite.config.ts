import { defineConfig } from "vite";

// Tauri ожидает статический frontendDist и фиксированный порт при dev
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "es2021",
    outDir: "dist",
  },
});
