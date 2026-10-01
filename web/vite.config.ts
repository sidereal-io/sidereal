import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // Keep earlier output, such as a cargo error from `just dev`, on screen.
  clearScreen: false,
  server: {
    // Stop with an error when 5173 is taken, instead of serving on another
    // port where a contributor would not look.
    strictPort: true,
    // The web shell needs no cross-origin access, so other local apps
    // cannot read the server through the proxy.
    cors: false,
    // The browser asks the web shell's own origin; the dev server forwards
    // to the v2 server.
    proxy: {
      '/healthz': 'http://localhost:5000',
    },
  },
})
