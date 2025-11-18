export async function checkDatabaseConnection(): Promise<{ connected: boolean }> {
  return { connected: true };
}
