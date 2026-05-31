import { useState, useMemo, useEffect } from "react";
import { useDebounce } from "@/hooks/use-debounce";
import { useQuery } from "@tanstack/react-query";
import { Header } from "@/components/header";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Search, Target, Clock, Image as ImageIcon, ChevronLeft, ChevronRight, ChevronDown, StickyNote, MapPin, ArrowUp, ArrowDown } from "lucide-react";
import { Popover, PopoverTrigger, PopoverContent } from "@/components/ui/popover";
import { Checkbox } from "@/components/ui/checkbox";
import { TargetDetailDialog } from "@/components/target-detail-dialog";
import { computeVisibility } from "../../../../packages/shared/src/visibility";
import type { CatalogObject, Location } from "../../../../packages/shared/src/types";
import { imageUrl } from "@shared/utils";

interface TargetSummary {
  targetName: string;
  imageCount: number;
  totalIntegrationHours: number;
  thumbnailImageId: number | null;
  objectType: string | null;
  constellation: string | null;
  vMag: number | null;
  commonNames: string | null;
  latestCaptureDate: string | null;
  imageIds: number[];
}

interface UserTargetData {
  catalogName: string;
  notes: string | null;
  tags: string[] | null;
}

interface BrowseResponse {
  items: CatalogObject[];
  total: number;
  page: number;
  pageSize: number;
}

const OBJECT_TYPE_LABELS: Record<string, string> = {
  G: "Galaxy", GGroup: "Galaxy Group", GPair: "Galaxy Pair", GTrpl: "Galaxy Triplet",
  GClstr: "Galaxy Cluster", PN: "Planetary Nebula", OCl: "Open Cluster", GCl: "Globular Cluster",
  HII: "HII Region", SNR: "Supernova Remnant", Neb: "Nebula", EmN: "Emission Nebula",
  RfN: "Reflection Nebula", DkN: "Dark Nebula", Cl: "Cluster", "*": "Star",
  "**": "Double Star", "*Ass": "Stellar Association", NonEx: "Non-Existent", Other: "Other", Dup: "Duplicate",
};

const MONTH_NAMES_SHORT = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

function getObjectTypeLabel(type: string | null): string {
  if (!type) return "Unknown";
  return OBJECT_TYPE_LABELS[type] || type;
}

export default function TargetsPage() {
  const [search, setSearch] = useState("");
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);
  const [constellationFilter, setConstellationFilter] = useState("all");
  const [locationId, setLocationId] = useState("none");
  const [showImaged, setShowImaged] = useState(false);
  const [showAnnotated, setShowAnnotated] = useState(false);
  const [messierOnly, setMessierOnly] = useState(false);
  const [minMag, setMinMag] = useState("");
  const [maxMag, setMaxMag] = useState("");
  const [minSize, setMinSize] = useState("");
  const [sortBy, setSortBy] = useState<"name" | "vMag" | "majorAxis" | "bestNow">("name");
  const [sortOrder, setSortOrder] = useState<"asc" | "desc">("asc");
  const [hideBelow, setHideBelow] = useState(false);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [page, setPage] = useState(1);
  const [selectedObject, setSelectedObject] = useState<CatalogObject | null>(null);
  const PAGE_SIZE = 50;

  // Debounce search input
  const debouncedSearch = useDebounce(search, 300);
  useEffect(() => {
    setPage(1);
  }, [debouncedSearch]);

  // Fetch locations (once)
  const { data: locations = [] } = useQuery<Location[]>({
    queryKey: ["/api/locations"],
  });

  // Fetch imaged targets (once)
  const { data: imagedTargets = [] } = useQuery<TargetSummary[]>({
    queryKey: ["/api/targets"],
  });

  // Fetch user annotations (once)
  const { data: userTargets = [] } = useQuery<UserTargetData[]>({
    queryKey: ["/api/user-targets"],
  });

  // Fetch catalog browse (paginated, server-filtered)
  const selectedLocation = useMemo(() => {
    if (locationId === "none") return null;
    return locations.find(l => l.id === parseInt(locationId)) || null;
  }, [locationId, locations]);

  const browseParams = new URLSearchParams();
  browseParams.set("page", String(page));
  browseParams.set("limit", String(PAGE_SIZE));
  if (debouncedSearch) browseParams.set("q", debouncedSearch);
  if (selectedTypes.length > 0) browseParams.set("type", selectedTypes.join(","));
  if (constellationFilter !== "all") browseParams.set("constellation", constellationFilter);
  if (minMag) browseParams.set("minMag", minMag);
  if (maxMag) browseParams.set("maxMag", maxMag);
  if (minSize) browseParams.set("minSize", minSize);
  if (messierOnly) browseParams.set("messierOnly", "true");
  if (selectedTags.length > 0) {
    const matchedNames = userTargets
      .filter(t => t.tags?.some(tag => selectedTags.includes(tag)))
      .map(t => t.catalogName);
    if (matchedNames.length > 0) {
      browseParams.set("names", matchedNames.join(","));
    } else {
      // No targets match these tags — set impossible filter to return 0 results
      browseParams.set("names", "__none__");
    }
  }
  if (sortBy !== "name") browseParams.set("sortBy", sortBy);
  if (sortOrder !== "asc" && sortBy !== "bestNow") browseParams.set("sortOrder", sortOrder);
  if (selectedLocation) {
    browseParams.set("latitude", String(selectedLocation.latitude));
    if (hideBelow) browseParams.set("hideBelow", "true");
  }

  const { data: browseData, isLoading: isBrowseLoading } = useQuery<BrowseResponse>({
    queryKey: [`/api/catalog/browse?${browseParams.toString()}`],
  });

  // Build lookup maps
  const imagedMap = useMemo(() => {
    const map = new Map<string, TargetSummary>();
    for (const t of imagedTargets) map.set(t.targetName, t);
    return map;
  }, [imagedTargets]);

  const userTargetMap = useMemo(() => {
    const map = new Map<string, UserTargetData>();
    for (const t of userTargets) map.set(t.catalogName, t);
    return map;
  }, [userTargets]);

  const allTags = useMemo(() => {
    const tags = new Set<string>();
    for (const t of userTargets) {
      if (t.tags) for (const tag of t.tags) tags.add(tag);
    }
    return Array.from(tags).sort();
  }, [userTargets]);

  // When "Imaged only" or "Annotated only" is active, use those datasets
  // directly instead of trying to filter the paginated browse results
  const useLocalData = showImaged || showAnnotated;

  const filteredItems = useMemo(() => {
    if (useLocalData) {
      // Build a list of imaged targets as pseudo-catalog objects for rendering
      let items = imagedTargets;

      if (showAnnotated) {
        items = items.filter(t => userTargetMap.has(t.targetName));
      }

      // Apply search filter
      if (debouncedSearch) {
        const q = debouncedSearch.toLowerCase();
        items = items.filter(t =>
          t.targetName.toLowerCase().includes(q) ||
          (t.commonNames && t.commonNames.toLowerCase().includes(q))
        );
      }
      if (selectedTypes.length > 0) {
        items = items.filter(t => t.objectType != null && selectedTypes.includes(t.objectType));
      }
      if (constellationFilter !== "all") {
        items = items.filter(t => t.constellation === constellationFilter);
      }

      // Convert to CatalogObject-like shape for consistent card rendering
      return items.map(t => ({
        id: 0,
        name: t.targetName,
        type: t.objectType,
        ra: null,
        dec: null,
        raDeg: null,
        decDeg: null,
        constellation: t.constellation,
        majorAxis: null,
        minorAxis: null,
        bMag: null,
        vMag: t.vMag,
        surfaceBrightness: null,
        hubbleType: null,
        messier: null,
        ngcRef: null,
        icRef: null,
        commonNames: t.commonNames,
        identifiers: null,
        createdAt: null,
      })) as CatalogObject[];
    }

    if (!browseData?.items) return [];
    let items = browseData.items;
    if (showAnnotated) {
      items = items.filter(obj => userTargetMap.has(obj.name));
    }
    return items;
  }, [browseData?.items, useLocalData, showImaged, showAnnotated, imagedMap, userTargetMap, imagedTargets, debouncedSearch, selectedTypes, constellationFilter]);

  // Server handles visibility filtering (hideBelow) and bestNow sorting,
  // so no client-side post-processing needed for those.
  const visibleItems = filteredItems;

  // Derive filter options from object types commonly present
  const COMMON_TYPES = ["G", "PN", "OCl", "GCl", "HII", "SNR", "Neb", "EmN", "RfN", "DkN", "*", "**"];

  // Summary stats
  const totalImaged = imagedTargets.length;
  const totalHours = imagedTargets.reduce((sum, t) => sum + t.totalIntegrationHours, 0);

  const totalResults = useLocalData ? visibleItems.length : (browseData?.total || 0);
  const totalPages = useLocalData ? 1 : Math.ceil(totalResults / PAGE_SIZE);
  const showFrom = totalResults > 0 ? (useLocalData ? 1 : (page - 1) * PAGE_SIZE + 1) : 0;
  const showTo = useLocalData ? totalResults : Math.min(page * PAGE_SIZE, totalResults);

  return (
    <div className="min-h-screen bg-background">
      <Header />
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Page Header */}
        <div className="flex items-center gap-3 mb-6">
          <Target className="h-7 w-7 text-primary" />
          <div>
            <h1 className="text-3xl font-bold">Targets</h1>
            <p className="text-muted-foreground text-sm">
              {totalResults.toLocaleString()} objects &middot; {totalImaged} imaged &middot; {Math.round(totalHours * 100) / 100} total hours
            </p>
          </div>
        </div>

        {/* Filters */}
        <div className="bg-card border border-border rounded-lg p-4 mb-6 space-y-3">
          <div className="flex flex-col sm:flex-row gap-3">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search catalog..."
                value={search}
                onChange={e => setSearch(e.target.value)}
                className="pl-9 sky-input"
              />
            </div>
            <Popover>
              <PopoverTrigger asChild>
                <Button variant="outline" className="w-full sm:w-[200px] sky-input justify-between font-normal">
                  {selectedTypes.length === 0
                    ? "All Types"
                    : selectedTypes.length === 1
                      ? getObjectTypeLabel(selectedTypes[0])
                      : `${selectedTypes.length} types`}
                  <ChevronDown className="ml-2 h-4 w-4 opacity-50" />
                </Button>
              </PopoverTrigger>
              <PopoverContent className="w-56 bg-gray-900 border-gray-700 p-2 max-h-72 overflow-y-auto">
                {COMMON_TYPES.map(type => (
                  <label key={type} className="flex items-center gap-2 px-2 py-1.5 text-sm hover:bg-gray-800 rounded cursor-pointer">
                    <Checkbox
                      checked={selectedTypes.includes(type)}
                      onCheckedChange={(checked) => {
                        setSelectedTypes(prev =>
                          checked ? [...prev, type] : prev.filter(t => t !== type)
                        );
                        setPage(1);
                      }}
                    />
                    {getObjectTypeLabel(type)}
                  </label>
                ))}
                {selectedTypes.length > 0 && (
                  <Button variant="ghost" size="sm" className="w-full mt-1 text-xs"
                    onClick={() => { setSelectedTypes([]); setPage(1); }}>
                    Clear
                  </Button>
                )}
              </PopoverContent>
            </Popover>
            <Select value={constellationFilter} onValueChange={v => { setConstellationFilter(v); setPage(1); }}>
              <SelectTrigger className="w-full sm:w-[160px] sky-input">
                <SelectValue placeholder="Constellation" />
              </SelectTrigger>
              <SelectContent className="bg-gray-900 border-gray-700 text-white max-h-60">
                <SelectItem value="all">All Constellations</SelectItem>
                {/* Common constellations - full list would be too large */}
                {["And","Aql","Aqr","Ara","Ari","Aur","Boo","CMa","CMi","CVn","Cam","Cap","Car","Cas","Cen","Cep","Cet","Col","Com","CrA","CrB","Cru","Cyg","Del","Dor","Dra","Eri","For","Gem","Gru","Her","Hor","Hya","Lac","Leo","Lep","Lib","Lup","Lyn","Lyr","Mon","Mus","Nor","Oph","Ori","Pav","Peg","Per","Phe","Pic","PsA","Psc","Pup","Pyx","Ret","Sco","Scl","Sct","Ser","Sex","Sgr","Tau","TrA","Tri","Tuc","UMa","UMi","Vel","Vir","Vol","Vul"].map(c => (
                  <SelectItem key={c} value={c}>{c}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            {locations.length > 0 && (
              <Select value={locationId} onValueChange={setLocationId}>
                <SelectTrigger className="w-full sm:w-[180px] sky-input">
                  <SelectValue placeholder="Location" />
                </SelectTrigger>
                <SelectContent className="bg-gray-900 border-gray-700 text-white">
                  <SelectItem value="none">No location</SelectItem>
                  {locations.map(loc => (
                    <SelectItem key={loc.id} value={String(loc.id)}>{loc.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
            {allTags.length > 0 && (
              <Popover>
                <PopoverTrigger asChild>
                  <Button variant="outline" className="w-full sm:w-[160px] sky-input justify-between font-normal">
                    {selectedTags.length === 0
                      ? "Tags"
                      : selectedTags.length === 1
                        ? selectedTags[0]
                        : `${selectedTags.length} tags`}
                    <ChevronDown className="ml-2 h-4 w-4 opacity-50" />
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-48 bg-gray-900 border-gray-700 p-2 max-h-60 overflow-y-auto">
                  {allTags.map(tag => (
                    <label key={tag} className="flex items-center gap-2 px-2 py-1.5 text-sm hover:bg-gray-800 rounded cursor-pointer">
                      <Checkbox
                        checked={selectedTags.includes(tag)}
                        onCheckedChange={(checked) => {
                          setSelectedTags(prev =>
                            checked ? [...prev, tag] : prev.filter(t => t !== tag)
                          );
                          setPage(1);
                        }}
                      />
                      {tag}
                    </label>
                  ))}
                  {selectedTags.length > 0 && (
                    <Button variant="ghost" size="sm" className="w-full mt-1 text-xs"
                      onClick={() => { setSelectedTags([]); setPage(1); }}>
                      Clear
                    </Button>
                  )}
                </PopoverContent>
              </Popover>
            )}
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <label className="flex items-center gap-1.5 text-sm cursor-pointer">
              <Checkbox checked={showImaged} onCheckedChange={() => { setShowImaged(!showImaged); setPage(1); }} />
              Imaged
            </label>
            <div className="h-5 w-px bg-border hidden sm:block" />
            <label className="flex items-center gap-1.5 text-sm cursor-pointer">
              <Checkbox checked={showAnnotated} onCheckedChange={() => { setShowAnnotated(!showAnnotated); setPage(1); }} />
              Annotated
            </label>
            <div className="h-5 w-px bg-border hidden sm:block" />
            <label className="flex items-center gap-1.5 text-sm cursor-pointer">
              <Checkbox checked={messierOnly} onCheckedChange={() => { setMessierOnly(!messierOnly); setPage(1); }} />
              Messier
            </label>
            {selectedLocation && (
              <>
                <div className="h-5 w-px bg-border hidden sm:block" />
                <label className="flex items-center gap-1.5 text-sm cursor-pointer">
                  <Checkbox checked={hideBelow} onCheckedChange={() => { setHideBelow(!hideBelow); setPage(1); }} />
                  Hide below horizon
                </label>
              </>
            )}
            <div className="h-5 w-px bg-border hidden sm:block" />
            <div className="flex items-center gap-1.5">
              <span className="text-xs text-muted-foreground whitespace-nowrap">Mag:</span>
              <Input
                type="number"
                placeholder="Min"
                value={minMag}
                onChange={e => { setMinMag(e.target.value); setPage(1); }}
                className="w-20 sky-input h-8 text-sm"
                step="0.1"
              />
              <span className="text-muted-foreground">&ndash;</span>
              <Input
                type="number"
                placeholder="Max"
                value={maxMag}
                onChange={e => { setMaxMag(e.target.value); setPage(1); }}
                className="w-20 sky-input h-8 text-sm"
                step="0.1"
              />
            </div>
            <div className="h-5 w-px bg-border hidden sm:block" />
            <div className="flex items-center gap-1.5">
              <span className="text-xs text-muted-foreground whitespace-nowrap">Size &ge;</span>
              <Input
                type="number"
                placeholder="arcmin"
                value={minSize}
                onChange={e => { setMinSize(e.target.value); setPage(1); }}
                className="w-24 sky-input h-8 text-sm"
                step="0.1"
              />
            </div>
            <div className="flex items-center gap-1.5 sm:ml-auto">
              <span className="text-xs text-muted-foreground">Sort:</span>
              <Select value={sortBy} onValueChange={v => { setSortBy(v as "name" | "vMag" | "majorAxis" | "bestNow"); setPage(1); }}>
                <SelectTrigger className="w-[140px] sky-input h-8 text-sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent className="bg-gray-900 border-gray-700 text-white">
                  <SelectItem value="name">Name</SelectItem>
                  <SelectItem value="vMag">Magnitude</SelectItem>
                  <SelectItem value="majorAxis">Size</SelectItem>
                  {selectedLocation && (
                    <SelectItem value="bestNow">Best tonight</SelectItem>
                  )}
                </SelectContent>
              </Select>
              <Button
                variant="outline"
                size="icon"
                className="h-8 w-8"
                onClick={() => { setSortOrder(o => o === "asc" ? "desc" : "asc"); setPage(1); }}
                title={sortOrder === "asc" ? "Ascending" : "Descending"}
              >
                {sortOrder === "asc"
                  ? <ArrowUp className="h-3.5 w-3.5" />
                  : <ArrowDown className="h-3.5 w-3.5" />}
              </Button>
            </div>
          </div>
        </div>

        {/* Card Grid */}
        {isBrowseLoading ? (
          <div className="text-center py-12 text-muted-foreground">Loading catalog...</div>
        ) : visibleItems.length === 0 ? (
          <div className="text-center py-12 text-muted-foreground">
            No objects match your filters.
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {visibleItems.map(obj => {
              const imaged = imagedMap.get(obj.name);
              const userTarget = userTargetMap.get(obj.name);
              const vis = selectedLocation && obj.raDeg != null && obj.decDeg != null
                ? computeVisibility(obj.raDeg, obj.decDeg, selectedLocation.latitude)
                : null;

              return (
                <Card
                  key={obj.name}
                  className="cursor-pointer hover:border-primary/50 transition-colors overflow-hidden"
                  onClick={() => setSelectedObject(obj)}
                >
                  {/* Thumbnail */}
                  <div className="h-36 bg-muted overflow-hidden">
                    {imaged?.thumbnailImageId != null ? (
                      <img
                        src={imageUrl(imaged.thumbnailImageId, 'thumbnail')}
                        alt={obj.name}
                        className="w-full h-full object-cover"
                      />
                    ) : obj.raDeg != null && obj.decDeg != null ? (
                      <img
                        src={`/api/catalog/thumbnail/${encodeURIComponent(obj.name)}`}
                        alt={obj.name}
                        className="w-full h-full object-cover"
                        loading="lazy"
                        onError={(e) => {
                          const target = e.currentTarget;
                          target.style.display = 'none';
                          target.parentElement!.classList.add('flex', 'items-center', 'justify-center');
                          const icon = document.createElement('div');
                          icon.className = 'text-muted-foreground opacity-30';
                          icon.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="1"/></svg>';
                          target.parentElement!.appendChild(icon);
                        }}
                      />
                    ) : (
                      <div className="w-full h-full flex items-center justify-center text-muted-foreground">
                        <Target className="h-8 w-8 opacity-30" />
                      </div>
                    )}
                  </div>

                  <CardContent className="p-4">
                    {/* Name */}
                    <h3 className="font-bold text-lg text-foreground leading-tight">
                      {obj.messier || obj.name}
                    </h3>
                    {obj.commonNames && (
                      <p className="text-sm text-muted-foreground mt-0.5 line-clamp-1">
                        {obj.commonNames}
                      </p>
                    )}
                    {obj.messier && (
                      <p className="text-xs text-muted-foreground">{obj.name}</p>
                    )}

                    {/* Badges */}
                    <div className="flex flex-wrap gap-1.5 mt-2">
                      {obj.type && (
                        <Badge variant="secondary" className="text-xs">
                          {getObjectTypeLabel(obj.type)}
                        </Badge>
                      )}
                      {obj.constellation && (
                        <Badge variant="outline" className="text-xs">{obj.constellation}</Badge>
                      )}
                      {obj.vMag != null && (
                        <Badge variant="outline" className="text-xs">mag {obj.vMag.toFixed(1)}</Badge>
                      )}
                      {imaged && (
                        <Badge className="text-xs bg-green-600/20 text-green-400 border-green-600/30">
                          Imaged
                        </Badge>
                      )}
                    </div>

                    {/* Visibility + stats row */}
                    <div className="flex items-center gap-3 mt-3 text-xs text-muted-foreground flex-wrap">
                      {imaged && (
                        <>
                          <span className="flex items-center gap-1">
                            <ImageIcon className="h-3.5 w-3.5" />
                            {imaged.imageCount}
                          </span>
                          <span className="flex items-center gap-1">
                            <Clock className="h-3.5 w-3.5" />
                            {imaged.totalIntegrationHours.toFixed(1)}h
                          </span>
                        </>
                      )}
                      {vis && vis.status !== 'never-rises' && (
                        <span className="flex items-center gap-1">
                          <MapPin className="h-3.5 w-3.5" />
                          {vis.status === 'circumpolar'
                            ? "Circumpolar"
                            : `Best: ${MONTH_NAMES_SHORT[vis.bestMonth - 1]}`}
                        </span>
                      )}
                      {vis && vis.status === 'never-rises' && (
                        <span className="text-red-400 text-xs">Below horizon</span>
                      )}
                    </div>

                    {/* User tags */}
                    {userTarget?.tags && userTarget.tags.length > 0 && (
                      <div className="flex flex-wrap gap-1 mt-2">
                        {userTarget.tags.map(tag => (
                          <Badge key={tag} variant="secondary" className="text-xs">{tag}</Badge>
                        ))}
                      </div>
                    )}
                    {userTarget?.notes && (
                      <div className="mt-1">
                        <StickyNote className="h-3.5 w-3.5 text-muted-foreground inline" />
                      </div>
                    )}
                  </CardContent>
                </Card>
              );
            })}
          </div>
        )}

        {/* Pagination */}
        {totalResults > 0 && (
          <div className="flex items-center justify-between mt-6 text-sm text-muted-foreground">
            <span>
              Showing {showFrom}–{showTo} of {totalResults.toLocaleString()}
            </span>
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={page <= 1}
                onClick={() => setPage(p => p - 1)}
              >
                <ChevronLeft className="h-4 w-4 mr-1" />
                Prev
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={page >= totalPages}
                onClick={() => setPage(p => p + 1)}
              >
                Next
                <ChevronRight className="h-4 w-4 ml-1" />
              </Button>
            </div>
          </div>
        )}
      </div>

      {/* Detail Dialog */}
      {selectedObject && (
        <TargetDetailDialog
          open={!!selectedObject}
          onOpenChange={open => { if (!open) setSelectedObject(null); }}
          catalogObject={selectedObject}
          imagedTarget={imagedMap.get(selectedObject.name)}
          userTarget={userTargetMap.get(selectedObject.name)}
          location={selectedLocation}
        />
      )}
    </div>
  );
}
