import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({ root: 'web', cacheDir: '/tmp/gxe-search-queue-vite-cache', plugins: [react()], build: { outDir: '../web-dist', emptyOutDir: true } });
