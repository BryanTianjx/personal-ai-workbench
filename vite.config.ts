import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri development server conventions: fixed port and strict host so the
// Tauri shell can point at a stable devUrl when it lands in E1-05.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: false,
  },
  build: {
    target: "chrome120",
    outDir: "dist",
  },
});
