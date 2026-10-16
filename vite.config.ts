import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({
  plugins: [react()],
  // Preserve Vite 6 output compatibility for existing desktop webviews.
  build: { target: ['es2020', 'edge88', 'firefox78', 'chrome87', 'safari14'] },
  server: { port: 1420, strictPort: true, proxy: { '/api': 'http://127.0.0.1:4173' } },
  clearScreen: false,
});
