import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { serve } from '@hono/node-server';
import { serveStatic } from '@hono/node-server/serve-static';
import path from 'path';
import fs from 'fs';
import { registerRoutes } from './routes';
import { WsManager } from './services/ws-manager';
import { cronManager, setWsManager as setCronWsManager } from './services/cron-manager';
import { setWsManager } from './services/astrometry';
import { workerManager } from './services/worker-manager';
import { catalogService } from './services/catalog';

const app = new Hono();

// Middleware
app.use('*', cors());

// Setup routes (wsManager set after server starts)
// We'll register routes after creating wsManager
let wsManager: WsManager;

// Main startup function
async function startServer() {
  const PORT = parseInt(process.env.PORT || '5000', 10);

  // Start Hono with @hono/node-server — returns a Node http.Server
  const server = serve({
    fetch: app.fetch,
    port: PORT,
  });

  // Create WebSocket manager attached to the Node HTTP server
  wsManager = new WsManager(server as any);

  // Register routes with wsManager
  registerRoutes(app, wsManager);

  // Serve static files (in production, frontend is pre-built to dist/public)
  const publicPath = path.resolve(process.cwd(), 'dist/public');

  app.use('/assets/*', serveStatic({ root: publicPath, rewriteRequestPath: (p) => p }));
  app.use('/favicon.ico', serveStatic({ root: publicPath, rewriteRequestPath: () => '/favicon.ico' }));
  app.use('/logo.png', serveStatic({ root: publicPath, rewriteRequestPath: () => '/logo.png' }));

  // SPA fallback: serve index.html for all non-API routes
  app.get('*', async (c) => {
    if (c.req.path.startsWith('/api/')) {
      return c.json({ message: 'Not found' }, 404);
    }
    const indexPath = path.join(publicPath, 'index.html');
    try {
      const html = fs.readFileSync(indexPath, 'utf8');
      return c.html(html);
    } catch {
      return c.text('Frontend not built. Run npm run build first.', 404);
    }
  });

  // Initialize cron manager
  cronManager.initialize();

  // Auto-load catalog if not yet loaded
  catalogService.getStatus().then(status => {
    if (!status.count) {
      console.log('Catalog not loaded, fetching OpenNGC catalog...');
      catalogService.loadCatalog()
        .then(result => console.log(`Catalog loaded: ${result.count} objects`))
        .catch(err => console.error('Failed to auto-load catalog:', err));
    } else {
      console.log(`Catalog already loaded: ${status.count} objects`);
    }
  }).catch(err => console.error('Failed to check catalog status:', err));

  // Set WebSocket manager in services for real-time updates
  setWsManager(wsManager);
  setCronWsManager(wsManager);

  // Start worker manager in production (in development, worker runs separately)
  if (process.env.NODE_ENV === 'production') {
    console.log('Starting worker manager...');
    workerManager.start().catch(error => {
      console.error('Failed to start worker manager:', error);
    });
  }

  console.log(`Sidereal server running on port ${PORT}`);
  console.log(`Environment: ${process.env.NODE_ENV || 'development'}`);
  console.log(`Database: ${process.env.DATABASE_URL ? 'PostgreSQL' : 'SQLite'}`);
  console.log(`Worker manager available in production mode`);
}

// Start the server
startServer().catch(console.error);

// Graceful shutdown handling
const gracefulShutdown = async (signal: string) => {
  console.log(`Received ${signal}. Starting graceful shutdown...`);

  // Stop worker manager
  if (process.env.NODE_ENV === 'production') {
    try {
      await workerManager.gracefulShutdown();
    } catch (error) {
      console.error('Error during worker shutdown:', error);
    }
  }

  // Stop cron manager
  try {
    cronManager.shutdown();
  } catch (error) {
    console.error('Error during cron manager shutdown:', error);
  }

  console.log('Graceful shutdown complete');
  process.exit(0);
};

// Handle shutdown signals
process.on('SIGTERM', () => gracefulShutdown('SIGTERM'));
process.on('SIGINT', () => gracefulShutdown('SIGINT'));

// Handle uncaught exceptions
process.on('uncaughtException', (error) => {
  console.error('Uncaught Exception:', error);
  gracefulShutdown('uncaughtException');
});

process.on('unhandledRejection', (reason, promise) => {
  console.error('Unhandled Rejection at:', promise, 'reason:', reason);
  gracefulShutdown('unhandledRejection');
});

// Export wsManager for use in other modules
export { wsManager };
