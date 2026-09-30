import react from "@vitejs/plugin-react";
import { env } from "node:process";
import { defineConfig } from "vite";

const host = env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: host || false,
    port: 5173,
    strictPort: true,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
  },
  build: {
    target: "es2022",
    outDir: "dist",
  },
});
