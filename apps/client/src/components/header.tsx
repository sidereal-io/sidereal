import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetTitle, SheetTrigger } from "@/components/ui/sheet";
import { Settings, Upload, Loader2, Menu } from "lucide-react";
import { Link, useLocation } from "wouter";
import { useState, useEffect } from "react";
import { useSocket } from "@/hooks/use-socket";
import { useToast } from "@/hooks/use-toast";
import { queryClient } from "@/lib/queryClient";
import { useQuery } from "@tanstack/react-query";
import type { Notification } from '@shared/types';

const NAV_LINKS = [
  { href: '/', label: 'Gallery' },
  { href: '/targets', label: 'Targets' },
  { href: '/equipment', label: 'Equipment' },
  { href: '/plate-solving', label: 'Plate Solving' },
  { href: '/sky-map', label: 'Sky Map' },
  { href: '/locations', label: 'Locations' },
] as const;

export function Header() {
  const [location] = useLocation();
  const [mobileOpen, setMobileOpen] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const socket = useSocket();
  const { toast } = useToast();

  const isActive = (path: string) => location === path;

  const { data: notifications = [] } = useQuery<Notification[]>({
    queryKey: ["/api/notifications"],
  });

  const refreshNotifications = () => {
    queryClient.invalidateQueries({ queryKey: ["/api/notifications"] });
  };

  useEffect(() => {
    if (!socket) return;

    socket.on('plate-solving-update', refreshNotifications);
    socket.on('source-sync-complete', refreshNotifications);

    return () => {
      socket.off('plate-solving-update', refreshNotifications);
      socket.off('source-sync-complete', refreshNotifications);
    };
  }, [socket]);

  const handleSync = async () => {
    setSyncing(true);
    try {
      const response = await fetch('/api/immich/sync-immich', {
        method: 'POST',
        credentials: 'include',
      });

      if (response.ok) {
        const data = await response.json().catch(() => ({}));
        toast({ title: 'Sync completed', description: data.message || 'Successfully synced images from Immich' });
        queryClient.invalidateQueries({ queryKey: ["/api/images"] });
        queryClient.invalidateQueries({ queryKey: ["/api/stats"] });
      } else {
        const errorData = await response.json().catch(() => ({}));
        toast({
          title: 'Sync failed',
          description: errorData.message || `Server returned ${response.status}`,
          variant: 'destructive',
        });
      }
    } catch (error) {
      toast({
        title: 'Sync failed',
        description: error instanceof Error ? error.message : 'Unknown error',
        variant: 'destructive',
      });
    } finally {
      setSyncing(false);
    }
  };

  const unacknowledgedCount = notifications.length;

  return (
    <header className="bg-card border-b border-border sticky top-0 z-40">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex justify-between items-center h-16">
          <Link href="/" className="flex items-center space-x-2 hover:opacity-80 transition-opacity">
            <img src="/logo.png" alt="Sidereal Logo" className="h-8 w-8" />
            <h1 className="text-xl font-bold text-foreground">Sidereal</h1>
          </Link>

          {/* Desktop navigation */}
          <nav className="hidden md:flex items-center space-x-6">
            {NAV_LINKS.map(({ href, label }) => (
              <Link
                key={href}
                href={href}
                className={`transition-colors ${isActive(href) ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {label}
              </Link>
            ))}
          </nav>

          {/* Right-side actions */}
          <div className="flex items-center space-x-3">
            <Button onClick={handleSync} disabled={syncing} className="sky-button-primary hidden md:inline-flex">
              {syncing ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Upload className="mr-2 h-4 w-4" />}
              {syncing ? 'Syncing...' : 'Sync Immich'}
            </Button>
            <a
              href="https://github.com/mstelz/sidereal"
              target="_blank"
              rel="noopener noreferrer"
              title="View on GitHub"
              className="hidden md:inline-flex"
            >
              <Button variant="ghost" size="icon">
                <svg className="h-5 w-5" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z"/></svg>
              </Button>
            </a>
            <Link href="/admin" className="hidden md:inline-flex">
              <Button
                variant="ghost"
                size="icon"
                className="relative"
                title={unacknowledgedCount > 0 ? `${unacknowledgedCount} unacknowledged notification${unacknowledgedCount > 1 ? 's' : ''}` : 'Admin Settings'}
              >
                <span className="relative inline-block">
                  <Settings className="h-7 w-7" />
                  {unacknowledgedCount > 0 && (
                    <span
                      className="absolute -top-1.5 -right-1.5 bg-red-600 text-white h-4 w-4 rounded-full flex items-center justify-center text-xs font-bold border-2 border-white"
                      style={{ minWidth: '1rem', minHeight: '1rem' }}
                    >
                      {unacknowledgedCount > 9 ? '9+' : unacknowledgedCount}
                    </span>
                  )}
                </span>
              </Button>
            </Link>

            {/* Mobile hamburger menu */}
            <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
              <SheetTrigger asChild>
                <Button variant="ghost" size="icon" className="md:hidden">
                  <Menu className="h-5 w-5" />
                  <span className="sr-only">Open menu</span>
                </Button>
              </SheetTrigger>
              <SheetContent side="right" className="w-64 p-0">
                <SheetTitle className="sr-only">Navigation</SheetTitle>
                <div className="flex items-center space-x-2 px-4 h-16 border-b border-border">
                  <img src="/logo.png" alt="Sidereal Logo" className="h-8 w-8" />
                  <span className="text-xl font-bold text-foreground">Sidereal</span>
                </div>
                <nav className="flex flex-col py-2">
                  {NAV_LINKS.map(({ href, label }) => (
                    <Link
                      key={href}
                      href={href}
                      onClick={() => setMobileOpen(false)}
                      className={`px-4 py-3 text-sm font-medium transition-colors ${
                        isActive(href)
                          ? 'bg-accent text-foreground'
                          : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground'
                      }`}
                    >
                      {label}
                    </Link>
                  ))}
                  <div className="border-t border-border my-2" />
                  <button
                    onClick={() => { setMobileOpen(false); handleSync(); }}
                    disabled={syncing}
                    className="px-4 py-3 text-sm font-medium transition-colors text-muted-foreground hover:bg-accent/50 hover:text-foreground flex items-center gap-2 text-left"
                  >
                    {syncing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Upload className="h-4 w-4" />}
                    {syncing ? 'Syncing...' : 'Sync Immich'}
                  </button>
                  <Link
                    href="/admin"
                    onClick={() => setMobileOpen(false)}
                    className={`px-4 py-3 text-sm font-medium transition-colors flex items-center justify-between ${
                      isActive('/admin')
                        ? 'bg-accent text-foreground'
                        : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground'
                    }`}
                  >
                    Admin Settings
                    {unacknowledgedCount > 0 && (
                      <span className="bg-red-600 text-white h-5 w-5 rounded-full flex items-center justify-center text-xs font-bold">
                        {unacknowledgedCount > 9 ? '9+' : unacknowledgedCount}
                      </span>
                    )}
                  </Link>
                </nav>
              </SheetContent>
            </Sheet>
          </div>
        </div>
      </div>
    </header>
  );
}
