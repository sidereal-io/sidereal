import { serve } from '@hono/node-server';
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import type { HealthStatus } from '@sidereal/shared-types';

const app = new Hono();

app.use('/*', cors({
  origin: process.env.FRONTEND_URL || 'http://localhost:3000',
  credentials: true,
}));

app.get('/health', async (c) => {
  const healthStatus: HealthStatus = {
    status: 'healthy',
    timestamp: new Date().toISOString(),
    service: 'backend',
  };

  return c.json(healthStatus);
});

const port = parseInt(process.env.PORT || '3001', 10);

console.log(`Backend running on port ${port}`);

serve({
  fetch: app.fetch,
  port,
});
