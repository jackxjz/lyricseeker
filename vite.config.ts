import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri 需要固定端口与固定地址，否则开发时窗口指向的 URL 与 dev server 不匹配。
export default defineConfig({
  plugins: [react()],
  // Tauri 的 dev server 不应被 Vite 清屏，避免盖住 Rust 编译输出。
  clearScreen: false,
  // Tauri 的 target 由 webview 决定，不必为了老浏览器降级。
  build: {
    target: "chrome110",
    sourcemap: false,
  },
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
