import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  root: 'ui',
  plugins: [react()],
  clearScreen: false,
  server: { host: 'localhost', port: 1420, strictPort: true },
  build: { outDir: '../dist', emptyOutDir: true, target: 'safari15' },
});
