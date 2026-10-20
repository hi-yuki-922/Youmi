/// <reference types="vitest/config" />
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { TanStackRouterVite } from "@tanstack/router-plugin/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [TanStackRouterVite(), react(), tailwindcss()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
      // monaco 导入直链真实文件：rolldown-vite 对 monaco 包子路径解析不可靠
      // （`?worker` 查询导入不转换、worker 子构建里解析失败、editor.api 时而
      // 解析失败，实测 8.3.3），alias 指文件绕开 exports 解析
      "monaco-api": fileURLToPath(
        new URL("./node_modules/monaco-editor/esm/vs/editor/editor.api.js", import.meta.url),
      ),
      "monaco-json-contribution": fileURLToPath(
        new URL(
          "./node_modules/monaco-editor/esm/vs/language/json/monaco.contribution.js",
          import.meta.url,
        ),
      ),
      "monaco-json-worker": fileURLToPath(
        new URL(
          "./node_modules/monaco-editor/esm/vs/language/json/json.worker.js",
          import.meta.url,
        ),
      ),
      "monaco-editor-worker": fileURLToPath(
        new URL("./node_modules/monaco-editor/esm/vs/editor/editor.worker.js", import.meta.url),
      ),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
    environment: "node",
  },
}));
