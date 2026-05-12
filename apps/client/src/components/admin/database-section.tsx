import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Database, Download, Loader2, HardDrive } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { useToast } from '@/hooks/use-toast';

interface DatabaseInfo {
  type: 'sqlite' | 'postgresql';
  path?: string;
  sizeBytes?: number;
  lastModified?: string;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

export function DatabaseSection() {
  const { toast } = useToast();
  const [downloading, setDownloading] = useState(false);

  const { data: dbInfo } = useQuery<DatabaseInfo>({
    queryKey: ['/api/admin/database'],
  });

  const handleBackup = async () => {
    setDownloading(true);
    try {
      const response = await fetch('/api/admin/database/backup');
      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.message || 'Backup failed');
      }

      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = response.headers.get('Content-Disposition')?.match(/filename="(.+)"/)?.[1] || 'sidereal-backup.db';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      toast({ title: 'Backup Downloaded', description: 'Database backup saved successfully.' });
    } catch (error) {
      toast({
        title: 'Backup Failed',
        description: error instanceof Error ? error.message : 'Failed to download backup',
        variant: 'destructive',
      });
    } finally {
      setDownloading(false);
    }
  };

  if (!dbInfo) return null;

  return (
    <Card className="mb-6">
      <CardHeader>
        <CardTitle className="flex items-center space-x-2">
          <Database className="h-5 w-5" />
          <span>Database</span>
        </CardTitle>
        <CardDescription>
          {dbInfo.type === 'sqlite'
            ? 'Built-in SQLite database — stored in the config directory'
            : 'Connected to an external PostgreSQL database'}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-muted/20 rounded-lg p-3 border">
            <div className="text-sm text-muted-foreground">Engine</div>
            <div className="text-lg font-bold capitalize">{dbInfo.type === 'sqlite' ? 'SQLite' : 'PostgreSQL'}</div>
          </div>
          {dbInfo.type === 'sqlite' && (
            <>
              <div className="bg-muted/20 rounded-lg p-3 border">
                <div className="text-sm text-muted-foreground">Database Size</div>
                <div className="text-lg font-bold">
                  <HardDrive className="inline h-4 w-4 mr-1 opacity-60" />
                  {dbInfo.sizeBytes != null ? formatBytes(dbInfo.sizeBytes) : 'Unknown'}
                </div>
              </div>
              <div className="bg-muted/20 rounded-lg p-3 border">
                <div className="text-sm text-muted-foreground">Last Modified</div>
                <div className="text-sm font-medium">
                  {dbInfo.lastModified
                    ? new Date(dbInfo.lastModified).toLocaleString()
                    : 'Unknown'}
                </div>
              </div>
            </>
          )}
        </div>

        {dbInfo.type === 'sqlite' && (
          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={downloading}
              onClick={handleBackup}
            >
              {downloading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Download className="mr-2 h-4 w-4" />}
              Download Backup
            </Button>
            <span className="text-sm text-muted-foreground">
              Downloads a copy of the SQLite database file
            </span>
          </div>
        )}

        {dbInfo.type === 'postgresql' && (
          <p className="text-sm text-muted-foreground">
            Use <code className="text-xs bg-muted px-1 py-0.5 rounded">pg_dump</code> to back up your PostgreSQL database.
          </p>
        )}
      </CardContent>
    </Card>
  );
}
