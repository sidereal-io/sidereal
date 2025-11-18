import { pgTable, serial, timestamp } from 'drizzle-orm/pg-core';

export const healthCheck = pgTable('health_check', {
  id: serial('id').primaryKey(),
  checkedAt: timestamp('checked_at').defaultNow().notNull(),
});
