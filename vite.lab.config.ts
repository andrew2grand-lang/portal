import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'
import { scanBridge } from './design-lab/server/scanBridge.ts'

// The Project Send design lab (npm run lab). Same packages, tokens and
// components as the app, served from design-lab/; never part of the app build.
// scanBridge runs the real portal-scan CLI for the lab's "scan a real folder".
export default defineConfig({
  root: 'design-lab',
  publicDir: '../public',
  plugins: [react(), scanBridge()],
  server: { port: 5190, strictPort: true, fs: { allow: ['..'] } },
  clearScreen: false,
})
