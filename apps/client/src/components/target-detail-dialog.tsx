import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import { Clock, Image as ImageIcon, MapPin, X, Plus } from "lucide-react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiRequest } from "@/lib/queryClient";
import type { CatalogObject } from "../../../../packages/shared/src/types";
import { computeVisibility } from "../../../../packages/shared/src/visibility";
import { imageUrl } from "@shared/utils";

interface TargetSummary {
  targetName: string;
  imageCount: number;
  totalIntegrationHours: number;
  thumbnailImageId: number | null;
  latestCaptureDate: string | null;
  imageIds: number[];
}

interface UserTargetData {
  catalogName: string;
  notes: string | null;
  tags: string[] | null;
}

interface TargetDetailDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  catalogObject: CatalogObject;
  imagedTarget?: TargetSummary;
  userTarget?: UserTargetData;
  location?: { latitude: number; longitude: number; name: string } | null;
}

const MONTH_NAMES = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

const PRESET_TAGS = ["Want to shoot", "Needs more data", "Priority", "Spring", "Summer", "Autumn", "Winter"];

const OBJECT_TYPE_LABELS: Record<string, string> = {
  G: "Galaxy", GGroup: "Galaxy Group", GPair: "Galaxy Pair", GTrpl: "Galaxy Triplet",
  GClstr: "Galaxy Cluster", PN: "Planetary Nebula", OCl: "Open Cluster", GCl: "Globular Cluster",
  HII: "HII Region", SNR: "Supernova Remnant", Neb: "Nebula", EmN: "Emission Nebula",
  RfN: "Reflection Nebula", DkN: "Dark Nebula", Cl: "Cluster", "*": "Star",
  "**": "Double Star", "*Ass": "Stellar Association", NonEx: "Non-Existent", Other: "Other", Dup: "Duplicate",
};

export function TargetDetailDialog({
  open,
  onOpenChange,
  catalogObject,
  imagedTarget,
  userTarget,
  location,
}: TargetDetailDialogProps) {
  const queryClient = useQueryClient();
  const [notes, setNotes] = useState(userTarget?.notes || "");
  const [tags, setTags] = useState<string[]>(userTarget?.tags || []);
  const [newTag, setNewTag] = useState("");
  const [isDirty, setIsDirty] = useState(false);

  useEffect(() => {
    setNotes(userTarget?.notes || "");
    setTags(userTarget?.tags || []);
    setIsDirty(false);
  }, [userTarget, catalogObject.name]);

  const saveMutation = useMutation({
    mutationFn: async () => {
      await apiRequest("PUT", `/api/user-targets/${encodeURIComponent(catalogObject.name)}`, {
        notes: notes || null,
        tags: tags.length > 0 ? tags : null,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["/api/user-targets"] });
      setIsDirty(false);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: async () => {
      await apiRequest("DELETE", `/api/user-targets/${encodeURIComponent(catalogObject.name)}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["/api/user-targets"] });
      setNotes("");
      setTags([]);
      setIsDirty(false);
    },
  });

  const visibility = location && catalogObject.raDeg != null && catalogObject.decDeg != null
    ? computeVisibility(catalogObject.raDeg, catalogObject.decDeg, location.latitude)
    : null;

  const addTag = (tag: string) => {
    const trimmed = tag.trim();
    if (trimmed && !tags.includes(trimmed)) {
      setTags([...tags, trimmed]);
      setIsDirty(true);
    }
    setNewTag("");
  };

  const removeTag = (tag: string) => {
    setTags(tags.filter(t => t !== tag));
    setIsDirty(true);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg max-h-[85vh] overflow-y-auto bg-card">
        <DialogHeader>
          <DialogTitle className="text-xl">
            {catalogObject.name}
            {catalogObject.commonNames && (
              <span className="text-sm font-normal text-muted-foreground ml-2">
                {catalogObject.commonNames}
              </span>
            )}
          </DialogTitle>
        </DialogHeader>

        {/* Survey thumbnail */}
        {!imagedTarget && catalogObject.raDeg != null && catalogObject.decDeg != null && (
          <div className="h-48 bg-muted rounded-md overflow-hidden -mt-2">
            <img
              src={`/api/catalog/thumbnail/${encodeURIComponent(catalogObject.name)}`}
              alt={catalogObject.name}
              className="w-full h-full object-cover"
              loading="lazy"
            />
          </div>
        )}
        {imagedTarget && imagedTarget.thumbnailImageId != null && (
          <div className="h-48 bg-muted rounded-md overflow-hidden -mt-2">
            <img
              src={imageUrl(imagedTarget.thumbnailImageId, 'thumbnail')}
              alt={catalogObject.name}
              className="w-full h-full object-cover"
            />
          </div>
        )}

        {/* Catalog details */}
        <div className="space-y-3">
          <div className="flex flex-wrap gap-1.5">
            {catalogObject.type && (
              <Badge variant="secondary">{OBJECT_TYPE_LABELS[catalogObject.type] || catalogObject.type}</Badge>
            )}
            {catalogObject.constellation && (
              <Badge variant="outline">{catalogObject.constellation}</Badge>
            )}
            {catalogObject.vMag != null && (
              <Badge variant="outline">mag {catalogObject.vMag.toFixed(1)}</Badge>
            )}
            {catalogObject.messier && (
              <Badge variant="secondary">{catalogObject.messier}</Badge>
            )}
          </div>

          {/* Coordinates */}
          {catalogObject.ra && catalogObject.dec && (
            <div className="text-xs text-muted-foreground">
              RA: {catalogObject.ra} &middot; Dec: {catalogObject.dec}
              {catalogObject.majorAxis && (
                <> &middot; Size: {catalogObject.majorAxis.toFixed(1)}
                  {catalogObject.minorAxis ? `×${catalogObject.minorAxis.toFixed(1)}` : ""}'</>
              )}
            </div>
          )}

          {catalogObject.identifiers && (
            <div className="text-xs text-muted-foreground">
              Also: {catalogObject.identifiers}
            </div>
          )}

          {/* Visibility */}
          {visibility && (
            <div className="bg-muted rounded-md p-3">
              <div className="flex items-center gap-2 mb-1">
                <MapPin className="h-4 w-4 text-muted-foreground" />
                <span className="text-sm font-medium">Visibility from {location!.name}</span>
              </div>
              <div className="text-sm text-muted-foreground">
                {visibility.status === 'never-rises' ? (
                  <span className="text-red-400">Never rises from this location</span>
                ) : visibility.status === 'circumpolar' ? (
                  <span className="text-green-400">Circumpolar — visible year-round</span>
                ) : (
                  <>
                    Prime season: {MONTH_NAMES[visibility.monthRange[0] - 1]} – {MONTH_NAMES[visibility.monthRange[1] - 1]}
                    {" "}&middot; Best: {MONTH_NAMES[visibility.bestMonth - 1]}
                  </>
                )}
                {visibility.status !== 'never-rises' && (
                  <> &middot; Max altitude: {Math.round(visibility.maxAltitude)}&deg;</>
                )}
              </div>
            </div>
          )}

          {/* Imaging stats */}
          {imagedTarget && (
            <div className="bg-muted rounded-md p-3">
              <div className="flex items-center gap-4 text-sm">
                <Badge className="bg-green-600/20 text-green-400 border-green-600/30">Imaged</Badge>
                <span className="flex items-center gap-1">
                  <ImageIcon className="h-3.5 w-3.5" />
                  {imagedTarget.imageCount} image{imagedTarget.imageCount !== 1 ? "s" : ""}
                </span>
                <span className="flex items-center gap-1">
                  <Clock className="h-3.5 w-3.5" />
                  {imagedTarget.totalIntegrationHours.toFixed(1)}h
                </span>
              </div>
              {imagedTarget.latestCaptureDate && (
                <div className="text-xs text-muted-foreground mt-1">
                  Latest: {new Date(imagedTarget.latestCaptureDate).toLocaleDateString()}
                </div>
              )}
            </div>
          )}

          {/* Notes */}
          <div>
            <label className="text-sm font-medium mb-1 block">Notes</label>
            <Textarea
              value={notes}
              onChange={e => { setNotes(e.target.value); setIsDirty(true); }}
              placeholder="Add notes about this target..."
              className="resize-none h-20 sky-input"
            />
          </div>

          {/* Tags */}
          <div>
            <label className="text-sm font-medium mb-1 block">Tags</label>
            <div className="flex flex-wrap gap-1.5 mb-2">
              {tags.map(tag => (
                <Badge key={tag} variant="secondary" className="gap-1">
                  {tag}
                  <button onClick={() => removeTag(tag)} className="hover:text-destructive">
                    <X className="h-3 w-3" />
                  </button>
                </Badge>
              ))}
            </div>
            <div className="flex gap-2">
              <Input
                value={newTag}
                onChange={e => setNewTag(e.target.value)}
                onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); addTag(newTag); } }}
                placeholder="Add tag..."
                className="sky-input flex-1"
              />
              <Button size="sm" variant="outline" onClick={() => addTag(newTag)} disabled={!newTag.trim()}>
                <Plus className="h-4 w-4" />
              </Button>
            </div>
            <div className="flex flex-wrap gap-1 mt-2">
              {PRESET_TAGS.filter(t => !tags.includes(t)).map(tag => (
                <button
                  key={tag}
                  onClick={() => addTag(tag)}
                  className="text-xs px-2 py-0.5 rounded-full border border-dashed border-muted-foreground/30 text-muted-foreground hover:border-primary hover:text-primary transition-colors"
                >
                  + {tag}
                </button>
              ))}
            </div>
          </div>

          {/* Actions */}
          <div className="flex gap-2 pt-2">
            <Button
              onClick={() => saveMutation.mutate()}
              disabled={!isDirty || saveMutation.isPending}
              className="flex-1"
            >
              {saveMutation.isPending ? "Saving..." : "Save"}
            </Button>
            {userTarget && (
              <Button
                variant="destructive"
                onClick={() => deleteMutation.mutate()}
                disabled={deleteMutation.isPending}
              >
                Remove
              </Button>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
