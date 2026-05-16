import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';

// Dev server proxies all backend-served paths to `cargo run` on its default
// port (18200). If you run the backend on a different port, change it here.
const API_TARGET = 'http://127.0.0.1:18200';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/admin': { target: API_TARGET, changeOrigin: true },
      '/api': { target: API_TARGET, changeOrigin: true },
      '/health': { target: API_TARGET, changeOrigin: true },
      '/config.js': { target: API_TARGET, changeOrigin: true },
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
  },
});
