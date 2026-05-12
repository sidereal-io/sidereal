import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Switch } from '@/components/ui/switch';
import { Checkbox } from '@/components/ui/checkbox';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import {
  Loader2,
  CheckCircle,
  XCircle,
} from 'lucide-react';
import { useToast } from '@/hooks/use-toast';
import type { AdminSettings, ImmichAlbum } from '@/pages/admin';

interface ImmichSectionProps {
  settings: AdminSettings;
  setSettings: React.Dispatch<React.SetStateAction<AdminSettings>>;
  albums: ImmichAlbum[];
  albumsLoading: boolean;
  albumError: string | null;
  immichTestStatus: 'idle' | 'testing' | 'success' | 'error';
  immichTestMessage: string;
  testImmichConnection: () => void;
}

export function ImmichSection({
  settings,
  setSettings,
  albums,
  albumsLoading,
  albumError,
  immichTestStatus,
  immichTestMessage,
  testImmichConnection,
}: ImmichSectionProps) {
  const { toast } = useToast();

  const getTestButtonIcon = (status: 'idle' | 'testing' | 'success' | 'error') => {
    switch (status) {
      case 'testing':
        return <Loader2 className="h-4 w-4 animate-spin" />;
      case 'success':
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'error':
        return <XCircle className="h-4 w-4 text-red-500" />;
      default:
        return null;
    }
  };

  return (
    <Card className="mb-6">
      <CardHeader>
        <CardTitle className="flex items-center">
          Immich Configuration
        </CardTitle>
        <CardDescription>
          Configure your Immich server connection for image synchronization
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <Label htmlFor="immichHost">Immich Server URL</Label>
            <Input
              id="immichHost"
              type="url"
              placeholder="https://your-immich-server.com"
              value={settings.immich.host}
              onChange={(e) => setSettings(prev => ({
                ...prev,
                immich: { ...prev.immich, host: e.target.value }
              }))}
            />
          </div>
          <div>
            <Label htmlFor="immichApiKey">API Key</Label>
            <Input
              id="immichApiKey"
              type="password"
              placeholder="Enter your Immich API key"
              value={settings.immich.apiKey}
              onChange={(e) => setSettings(prev => ({
                ...prev,
                immich: { ...prev.immich, apiKey: e.target.value }
              }))}
            />
          </div>
        </div>

        <div className="flex items-center space-x-2">
          <Switch
            id="immichAutoSync"
            checked={settings.immich.autoSync}
            onCheckedChange={(checked) => setSettings(prev => ({
              ...prev,
              immich: { ...prev.immich, autoSync: checked }
            }))}
          />
          <Label htmlFor="immichAutoSync">Enable automatic synchronization</Label>
        </div>

        {/* Sync by album toggle */}
        <div className="flex items-center space-x-2">
          <Switch
            id="immichSyncByAlbum"
            checked={settings.immich.syncByAlbum}
            onCheckedChange={(checked) => setSettings(prev => ({
              ...prev,
              immich: { ...prev.immich, syncByAlbum: checked }
            }))}
          />
          <Label htmlFor="immichSyncByAlbum">Sync by album only</Label>
        </div>

        {/* Album selection if syncByAlbum is enabled */}
        {settings.immich.syncByAlbum && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <Label htmlFor="immichAlbums">Select albums to sync</Label>
              {albums.length > 0 && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs"
                  onClick={() => {
                    const allIds = albums.map(a => a.id);
                    const isAllSelected = settings.immich.selectedAlbumIds.length === albums.length;
                    setSettings(prev => ({
                      ...prev,
                      immich: {
                        ...prev.immich,
                        selectedAlbumIds: isAllSelected ? [] : allIds
                      }
                    }));
                  }}
                >
                  {settings.immich.selectedAlbumIds.length === albums.length ? "Deselect All" : "Select All"}
                </Button>
              )}
            </div>

            {albumsLoading ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground p-4 bg-muted/20 rounded-md border border-dashed">
                <Loader2 className="h-4 w-4 animate-spin" />
                Loading albums from Immich...
              </div>
            ) : albumError ? (
              <div className="text-sm text-red-500 p-4 bg-red-500/10 rounded-md border border-red-500/20">
                {albumError}
              </div>
            ) : albums.length === 0 ? (
              <div className="text-sm text-muted-foreground p-4 bg-muted/20 rounded-md border border-dashed">
                No albums found on Immich server.
              </div>
            ) : (
              <ScrollArea className="h-[200px] w-full rounded-md border bg-gray-900 border-gray-700 p-4">
                <div className="space-y-3">
                  {albums.map((album) => (
                    <div key={album.id} className="flex items-center space-x-3">
                      <Checkbox
                        id={`album-${album.id}`}
                        checked={settings.immich.selectedAlbumIds.includes(album.id)}
                        onCheckedChange={(checked) => {
                          setSettings(prev => {
                            const selected = checked
                              ? [...prev.immich.selectedAlbumIds, album.id]
                              : prev.immich.selectedAlbumIds.filter(id => id !== album.id);
                            return {
                              ...prev,
                              immich: { ...prev.immich, selectedAlbumIds: selected }
                            };
                          });
                        }}
                      />
                      <Label
                        htmlFor={`album-${album.id}`}
                        className="text-sm font-normal text-gray-200 cursor-pointer"
                      >
                        {album.albumName}
                      </Label>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
            {/* Validation: must select at least one album */}
            {settings.immich.syncByAlbum && settings.immich.selectedAlbumIds.length === 0 && (
              <div className="text-xs text-red-500 mt-1">You must select at least one album to sync.</div>
            )}
          </div>
        )}

        {settings.immich.autoSync && (
          <div>
            <Label htmlFor="immichSyncFrequency">Sync Frequency (Cron Expression)</Label>
            <Input
              id="immichSyncFrequency"
              placeholder="0 */4 * * *"
              value={settings.immich.syncFrequency}
              onChange={(e) => setSettings(prev => ({
                ...prev,
                immich: { ...prev.immich, syncFrequency: e.target.value }
              }))}
            />
            <p className="text-sm text-muted-foreground mt-1">
              Example: "0 */4 * * *" = every 4 hours.
              <a
                href="https://crontab.guru"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline ml-1"
              >
                Learn more
              </a>
            </p>
          </div>
        )}

        <div className="flex items-center justify-between">
          <div className="flex-1">
            {immichTestMessage && (
              <p className={`text-sm ${immichTestStatus === 'error' ? 'text-red-600' : 'text-green-600'}`}>
                {immichTestMessage}
              </p>
            )}
          </div>
          <Button
            onClick={testImmichConnection}
            disabled={immichTestStatus === 'testing' || !settings.immich.host.trim() || !settings.immich.apiKey.trim()}
            size="sm"
            variant="outline"
          >
            {getTestButtonIcon(immichTestStatus)}
            Test Connection
          </Button>
        </div>

        <Separator />

        {/* Metadata Sync to Immich */}
        <div className="space-y-4">
          <div className="flex items-center space-x-2">
            <Switch
              id="metadataSyncEnabled"
              checked={settings.immich.metadataSyncEnabled}
              onCheckedChange={(checked) => setSettings(prev => ({
                ...prev,
                immich: { ...prev.immich, metadataSyncEnabled: checked }
              }))}
            />
            <Label htmlFor="metadataSyncEnabled">Enable metadata writeback to Immich</Label>
          </div>
          <p className="text-sm text-muted-foreground">
            When enabled, Sidereal metadata (description, coordinates, tags) can be synced back to your Immich assets.
          </p>

          {settings.immich.metadataSyncEnabled && (
            <div className="space-y-3 pl-6 border-l-2 border-border">
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="syncDescription"
                  checked={settings.immich.syncDescription}
                  onCheckedChange={(checked) => setSettings(prev => ({
                    ...prev,
                    immich: { ...prev.immich, syncDescription: !!checked }
                  }))}
                />
                <Label htmlFor="syncDescription" className="text-sm font-normal">Sync image description</Label>
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="syncCoordinates"
                  checked={settings.immich.syncCoordinates}
                  onCheckedChange={(checked) => setSettings(prev => ({
                    ...prev,
                    immich: { ...prev.immich, syncCoordinates: !!checked }
                  }))}
                />
                <Label htmlFor="syncCoordinates" className="text-sm font-normal">Sync GPS coordinates (latitude, longitude)</Label>
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="syncTags"
                  checked={settings.immich.syncTags}
                  onCheckedChange={(checked) => setSettings(prev => ({
                    ...prev,
                    immich: { ...prev.immich, syncTags: !!checked }
                  }))}
                />
                <Label htmlFor="syncTags" className="text-sm font-normal">Sync tags to Immich (includes object type, constellation, equipment names)</Label>
              </div>

              <div className="pt-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={async () => {
                    try {
                      toast({ title: "Syncing...", description: "Syncing all metadata to Immich" });
                      const response = await fetch('/api/immich/sync-metadata-all', { method: 'POST' });
                      const data = await response.json();
                      if (response.ok) {
                        toast({ title: "Sync Complete", description: data.message });
                      } else {
                        toast({ title: "Sync Failed", description: data.message, variant: "destructive" });
                      }
                    } catch {
                      toast({ title: "Error", description: "Failed to sync metadata", variant: "destructive" });
                    }
                  }}
                  disabled={!settings.immich.host.trim() || !settings.immich.apiKey.trim()}
                >
                  Sync All Metadata to Immich
                </Button>
              </div>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
