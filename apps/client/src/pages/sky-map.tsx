import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Loader2, X, ExternalLink, Minimize2, Maximize2, Telescope, Camera, Eye, EyeOff } from "lucide-react";
import { Header } from "@/components/header";
import { Link } from "wouter";
import type { Equipment } from "@shared/schema";
import { imageUrl } from "@shared/utils";

declare global {
  interface Window {
    A: any;
  }
}

interface SkyMapMarker {
  id: number;
  title: string;
  ra: string;
  dec: string;
  objectType: string | null;
  constellation: string | null;
  fieldOfView: string | null;
}

interface FovResult { widthDeg: number; heightDeg: number }

function calculateFov(
  telescope: Equipment,
  camera: Equipment
): FovResult | null {
  const tSpecs = telescope.specifications as Record<string, unknown> | null;
  const cSpecs = camera.specifications as Record<string, unknown> | null;
  if (!tSpecs || !cSpecs) return null;

  const focalLength = typeof tSpecs.focalLength === "number" ? tSpecs.focalLength : null;
  const pixelSize = typeof cSpecs.pixelSize === "number" ? cSpecs.pixelSize : null;
  const resolution = typeof cSpecs.resolution === "string" ? cSpecs.resolution : null;
  if (!focalLength || !pixelSize || !resolution) return null;

  const match = resolution.match(/^(\d+)\s*[xX×]\s*(\d+)$/);
  if (!match) return null;

  const hPixels = parseInt(match[1], 10);
  const vPixels = parseInt(match[2], 10);
  const sensorWidthMm = (pixelSize * hPixels) / 1000;
  const sensorHeightMm = (pixelSize * vPixels) / 1000;
  const widthDeg = 2 * Math.atan(sensorWidthMm / (2 * focalLength)) * (180 / Math.PI);
  const heightDeg = 2 * Math.atan(sensorHeightMm / (2 * focalLength)) * (180 / Math.PI);
  return { widthDeg, heightDeg };
}

function getMissingFovSpecs(telescope: Equipment, camera: Equipment): string[] {
  const missing: string[] = [];
  const tSpecs = telescope.specifications as Record<string, unknown> | null;
  const cSpecs = camera.specifications as Record<string, unknown> | null;
  if (!tSpecs || typeof tSpecs.focalLength !== "number") missing.push(`${telescope.name}: focal length`);
  if (!cSpecs || typeof cSpecs.pixelSize !== "number") missing.push(`${camera.name}: pixel size`);
  if (!cSpecs || typeof cSpecs.resolution !== "string" || !/^\d+\s*[xX×]\s*\d+$/.test(cSpecs.resolution as string))
    missing.push(`${camera.name}: resolution`);
  return missing;
}

export default function SkyMapPage() {
  const containerRef = useRef<HTMLDivElement>(null);
  const aladinRef = useRef<any>(null);
  const fovOverlayRef = useRef<any>(null);
  const [scriptLoaded, setScriptLoaded] = useState(false);
  const [scriptError, setScriptError] = useState(false);
  const [webglError, setWebglError] = useState(false);
  const [selectedMarker, setSelectedMarker] = useState<SkyMapMarker | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [selectedTelescopeId, setSelectedTelescopeId] = useState<string | undefined>(undefined);
  const [selectedCameraId, setSelectedCameraId] = useState<string | undefined>(undefined);
  const [fovEnabled, setFovEnabled] = useState(true);

  // Detect Aladin Lite's CSS-based fullscreen (adds "aladin-fullscreen" class to the container)
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new MutationObserver(() => {
      setIsFullscreen(container.classList.contains("aladin-fullscreen"));
    });
    observer.observe(container, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, [scriptLoaded]);

  const { data: markers = [], isLoading } = useQuery<SkyMapMarker[]>({
    queryKey: ["/api/sky-map/markers"],
  });

  const { data: equipmentList = [] } = useQuery<Equipment[]>({
    queryKey: ["/api/equipment"],
  });

  const telescopes = useMemo(
    () => equipmentList.filter((e) => e.type === "telescope"),
    [equipmentList]
  );
  const cameras = useMemo(
    () => equipmentList.filter((e) => e.type === "camera"),
    [equipmentList]
  );

  const selectedTelescope = useMemo(
    () => telescopes.find((t) => String(t.id) === selectedTelescopeId) ?? null,
    [telescopes, selectedTelescopeId]
  );
  const selectedCamera = useMemo(
    () => cameras.find((c) => String(c.id) === selectedCameraId) ?? null,
    [cameras, selectedCameraId]
  );

  const fovResult = useMemo(() => {
    if (!selectedTelescope || !selectedCamera) return null;
    return calculateFov(selectedTelescope, selectedCamera);
  }, [selectedTelescope, selectedCamera]);

  // Check WebGL2 availability
  useEffect(() => {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl2");
    if (!gl) {
      setWebglError(true);
    }
  }, []);

  // Load Aladin Lite script
  useEffect(() => {
    if (webglError) return;
    if (window.A) {
      if (window.A.init) {
        window.A.init.then(() => setScriptLoaded(true));
      } else {
        setScriptLoaded(true);
      }
      return;
    }

    const script = document.createElement("script");
    script.src = "https://aladin.cds.unistra.fr/AladinLite/api/v3/latest/aladin.js";
    script.charset = "utf-8";
    script.onload = () => {
      if (window.A && window.A.init) {
        window.A.init.then(() => setScriptLoaded(true));
      } else {
        setScriptLoaded(true);
      }
    };
    script.onerror = () => setScriptError(true);
    document.body.appendChild(script);

    return () => {
      if (script.parentNode) {
        document.body.removeChild(script);
      }
    };
  }, []);

  const handleObjectClicked = useCallback((object: any) => {
    if (object && object.data) {
      setSelectedMarker(object.data as SkyMapMarker);
    } else {
      setSelectedMarker(null);
    }
  }, []);

  // Initialize Aladin and plot markers
  useEffect(() => {
    if (!scriptLoaded || !containerRef.current || isLoading || webglError) return;

    if (!aladinRef.current) {
      try {
        aladinRef.current = window.A.aladin(containerRef.current, {
          survey: "P/DSS2/color",
          fov: 180,
          projection: "AIT",
          showZoomControl: true,
          showLayersControl: true,
        });
        aladinRef.current.on("objectClicked", handleObjectClicked);
      } catch (err) {
        console.error("Aladin initialization failed:", err);
        setScriptError(true);
        return;
      }
    }

    const aladin = aladinRef.current;
    if (!aladin) return;

    // Clear existing catalogs
    const catalogs = aladin.view?.catalogs;
    if (catalogs) {
      while (catalogs.length > 0) {
        aladin.removeCatalog(catalogs[0]);
      }
    }

    if (markers.length === 0) return;

    const catalog = window.A.catalog({
      name: "My Images",
      sourceSize: 18,
      color: "#3b82f6",
      onClick: "showPopup",
    });
    aladin.addCatalog(catalog);

    const sources = markers
      .map((marker) => {
        const ra = parseFloat(marker.ra);
        const dec = parseFloat(marker.dec);
        if (isNaN(ra) || isNaN(dec)) return null;

        const source = window.A.marker(ra, dec, {
          popupTitle: marker.title,
          popupDesc: "",
        });
        source.data = marker;
        return source;
      })
      .filter(Boolean);

    catalog.addSources(sources);
  }, [scriptLoaded, markers, isLoading, handleObjectClicked]);

  // FOV overlay effect
  useEffect(() => {
    const aladin = aladinRef.current;
    if (!aladin || !scriptLoaded) return;

    // Clear previous overlay footprints
    if (fovOverlayRef.current) {
      try {
        fovOverlayRef.current.removeAll();
      } catch {
        // best-effort
      }
    }

    if (!fovEnabled || !fovResult) return;

    // Create overlay once, reuse across re-renders
    if (!fovOverlayRef.current) {
      const overlay = window.A.graphicOverlay({ color: "#22d3ee", lineWidth: 2 });
      aladin.addOverlay(overlay);
      fovOverlayRef.current = overlay;
    }

    const overlay = fovOverlayRef.current;
    let cancelled = false;

    const drawFovBox = (ra: number, dec: number) => {
      if (cancelled) return;
      try {
        overlay.removeAll();
      } catch {
        // best-effort
      }
      const cosDec = Math.cos((dec * Math.PI) / 180);
      const halfW = fovResult.widthDeg / 2;
      const halfH = fovResult.heightDeg / 2;
      const raCorr = cosDec > 0.001 ? halfW / cosDec : halfW;
      overlay.addFootprints([
        window.A.polygon(
          [
            [ra - raCorr, dec - halfH],
            [ra + raCorr, dec - halfH],
            [ra + raCorr, dec + halfH],
            [ra - raCorr, dec + halfH],
          ],
          { color: "#22d3ee", lineWidth: 2 }
        ),
      ]);
    };

    // Draw at current center
    try {
      const [ra, dec] = aladin.getRaDec();
      drawFovBox(ra, dec);
    } catch {
      // getRaDec may not be available yet; wait for positionChanged
    }

    // Redraw on pan/zoom — positionChanged passes an object with ra/dec
    const onPositionChanged = (pos: any) => {
      if (pos && typeof pos.ra === "number" && typeof pos.dec === "number") {
        drawFovBox(pos.ra, pos.dec);
      } else {
        try {
          const [ra, dec] = aladin.getRaDec();
          drawFovBox(ra, dec);
        } catch {
          // give up
        }
      }
    };
    aladin.on("positionChanged", onPositionChanged);

    return () => {
      cancelled = true;
      try {
        overlay.removeAll();
      } catch {
        // best-effort
      }
    };
  }, [scriptLoaded, fovEnabled, fovResult]);

  const hasEquipment = telescopes.length > 0 || cameras.length > 0;

  return (
    <div className="dark min-h-screen bg-background text-foreground">
      {!isFullscreen && <Header />}
      <main className="flex flex-col" style={{ height: isFullscreen ? "100vh" : "calc(100vh - 4rem)" }}>
        {!isFullscreen && (
          <div className="px-4 sm:px-6 lg:px-8 py-4 flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold">Sky Map</h1>
              <p className="text-muted-foreground text-sm">
                {markers.length > 0
                  ? `${markers.length} object${markers.length !== 1 ? "s" : ""} plotted`
                  : "Visualize your plate-solved images on the sky"}
              </p>
            </div>
          </div>
        )}

        <div className={`flex-1 relative ${isFullscreen ? "" : "mx-4 sm:mx-6 lg:mx-8 mb-4 rounded-lg border border-border"} overflow-hidden`}>
          {isLoading ? (
            <div className="absolute inset-0 flex items-center justify-center bg-background/80 z-10">
              <div className="flex flex-col items-center gap-2">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                <p className="text-sm text-muted-foreground">Loading markers...</p>
              </div>
            </div>
          ) : markers.length === 0 ? (
            <div className="absolute inset-0 flex items-center justify-center bg-background/80 z-10">
              <div className="text-center">
                <p className="text-muted-foreground font-medium">No plate-solved images to display</p>
                <p className="text-sm text-muted-foreground mt-1">
                  <Link href="/plate-solving" className="text-primary hover:underline">
                    Plate solve your images
                  </Link>
                  {" "}to see them on the sky map.
                </p>
              </div>
            </div>
          ) : webglError ? (
            <div className="absolute inset-0 flex items-center justify-center bg-background z-10 text-center p-6">
              <div className="max-w-md mx-auto">
                <p className="text-destructive font-medium text-lg">WebGL2 Not Supported</p>
                <p className="text-sm text-muted-foreground mt-2">
                  The Sky Map requires WebGL2 to render the star atlas. Please try a modern browser like Chrome, Firefox, or Edge, and ensure hardware acceleration is enabled.
                </p>
              </div>
            </div>
          ) : scriptError ? (
            <div className="absolute inset-0 flex items-center justify-center bg-background z-10">
              <div className="text-center">
                <p className="text-destructive font-medium">Failed to load sky atlas</p>
                <p className="text-sm text-muted-foreground mt-1">
                  Could not load Aladin Lite from CDN. Check your internet connection.
                </p>
              </div>
            </div>
          ) : !scriptLoaded ? (
            <div className="absolute inset-0 flex items-center justify-center bg-background/80 z-10">
              <div className="flex flex-col items-center gap-2">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                <p className="text-sm text-muted-foreground">Loading sky atlas...</p>
              </div>
            </div>
          ) : null}

          <div
            ref={containerRef}
            id="aladin-container"
            className="w-full h-full"
          />

          {/* Equipment selector - bottom left, avoids Aladin's built-in top controls */}
          {hasEquipment && scriptLoaded && (
            <div className="absolute top-4 left-4 z-50 flex flex-col gap-2 w-48">
              {telescopes.length > 0 && (
                <Select value={selectedTelescopeId} onValueChange={setSelectedTelescopeId}>
                  <SelectTrigger className="h-8 text-xs bg-background/80 backdrop-blur-sm border-border/50">
                    <Telescope className="h-3.5 w-3.5 mr-1.5 shrink-0" />
                    <SelectValue placeholder="Telescope" />
                  </SelectTrigger>
                  <SelectContent>
                    {telescopes.map((t) => (
                      <SelectItem key={t.id} value={String(t.id)}>
                        {t.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}

              {cameras.length > 0 && (
                <Select value={selectedCameraId} onValueChange={setSelectedCameraId}>
                  <SelectTrigger className="h-8 text-xs bg-background/80 backdrop-blur-sm border-border/50">
                    <Camera className="h-3.5 w-3.5 mr-1.5 shrink-0" />
                    <SelectValue placeholder="Camera" />
                  </SelectTrigger>
                  <SelectContent>
                    {cameras.map((c) => (
                      <SelectItem key={c.id} value={String(c.id)}>
                        {c.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}

              {selectedTelescope && selectedCamera && (
                <div className="flex items-center gap-1.5">
                  <Button
                    variant="secondary"
                    size="sm"
                    className="h-8 text-xs opacity-80 hover:opacity-100 flex-1"
                    onClick={() => setFovEnabled(!fovEnabled)}
                  >
                    {fovEnabled ? (
                      <Eye className="h-3.5 w-3.5 mr-1.5" />
                    ) : (
                      <EyeOff className="h-3.5 w-3.5 mr-1.5" />
                    )}
                    FOV {fovEnabled ? "On" : "Off"}
                  </Button>
                  {fovResult && fovEnabled && (
                    <Badge variant="secondary" className="text-[10px] px-1.5 py-0.5 whitespace-nowrap bg-background/80 backdrop-blur-sm">
                      {(fovResult.widthDeg * 60).toFixed(1)}' × {(fovResult.heightDeg * 60).toFixed(1)}'
                    </Badge>
                  )}
                </div>
              )}

              {selectedTelescope && selectedCamera && !fovResult && (() => {
                const missing = getMissingFovSpecs(selectedTelescope, selectedCamera);
                return (
                  <p className="text-[10px] text-amber-400/80 leading-tight">
                    Missing specs — {missing.join(", ")}
                  </p>
                );
              })()}
            </div>
          )}

          <Button
            variant="secondary"
            size="sm"
            className="absolute top-4 right-4 z-50 opacity-80 hover:opacity-100"
            onClick={() => {
              // Click Aladin's hidden fullscreen toggle to enter/exit
              const btn = containerRef.current?.querySelector<HTMLButtonElement>(
                ".aladin-fullScreen-control .aladin-btn"
              );
              btn?.click();
            }}
          >
            {isFullscreen ? (
              <>
                <Minimize2 className="h-4 w-4 mr-1.5" />
                Exit Fullscreen
              </>
            ) : (
              <>
                <Maximize2 className="h-4 w-4 mr-1.5" />
                Fullscreen
              </>
            )}
          </Button>

          {selectedMarker && (
            <Card className="absolute top-4 right-4 z-20 w-72 shadow-lg">
              <CardContent className="p-4">
                <div className="flex justify-between items-start mb-2">
                  <h3 className="font-semibold text-sm leading-tight pr-2">
                    {selectedMarker.title}
                  </h3>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 shrink-0"
                    onClick={() => setSelectedMarker(null)}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>

                {selectedMarker.id && (
                  <img
                    src={imageUrl(selectedMarker.id, 'thumbnail')}
                    alt={selectedMarker.title}
                    className="w-full h-36 object-cover rounded-md mb-3"
                  />
                )}

                <div className="flex flex-wrap gap-1.5 mb-3">
                  {selectedMarker.objectType && (
                    <Badge variant="secondary">{selectedMarker.objectType}</Badge>
                  )}
                  {selectedMarker.constellation && (
                    <Badge variant="outline">{selectedMarker.constellation}</Badge>
                  )}
                </div>

                {selectedMarker.fieldOfView && (
                  <p className="text-xs text-muted-foreground mb-3">
                    Field of view: {selectedMarker.fieldOfView}
                  </p>
                )}

                <Link
                  href={`/?image=${selectedMarker.id}`}
                  className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline"
                >
                  <ExternalLink className="h-3.5 w-3.5" />
                  View in Gallery
                </Link>
              </CardContent>
            </Card>
          )}
        </div>
      </main>
    </div>
  );
}
