import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Heart, Share, Plus, Settings } from "lucide-react";
import type { AstroImage, Equipment } from "@shared/schema";
import { imageUrl } from "@shared/utils";
import { RemoteImage } from "./remote-image";

interface ImageGalleryProps {
  images: AstroImage[];
  equipment: Equipment[];
  onImageClick: (image: AstroImage) => void;
  isLoading: boolean;
  hasMoreImages?: boolean;
  onLoadMore?: () => void;
  totalImages?: number;
  visibleCount?: number;
}

export function ImageGallery({ 
  images, 
  equipment, 
  onImageClick, 
  isLoading,
  hasMoreImages = false,
  onLoadMore,
  totalImages = 0,
  visibleCount = 0
}: ImageGalleryProps) {
  const getStatusBadge = (image: AstroImage) => {
    if (image.plateSolved) {
      return <Badge className="status-plate-solved text-xs px-1.5 py-0.5 text-[10px]">Plate Solved</Badge>;
    }
    return <Badge className="status-no-data text-xs px-1.5 py-0.5 text-[10px]">No Plate Data</Badge>;
  };

  const formatExposureData = (image: AstroImage) => {
    const parts = [];
    if (image.focalLength) parts.push(`${image.focalLength}mm`);
    if (image.aperture) parts.push(image.aperture);
    if (image.iso) parts.push(`ISO ${image.iso}`);
    if (image.exposureTime) parts.push(image.exposureTime);
    return parts.join(' • ');
  };

  const formatIntegrationData = (image: AstroImage) => {
    const parts = [];
    if (image.frameCount) parts.push(`${image.frameCount} frames`);
    if (image.totalIntegration) parts.push(`${image.totalIntegration}h total`);
    return parts.join(' • ');
  };

  const formatDate = (date: Date | null) => {
    if (!date) return "Unknown date";
    return new Date(date).toLocaleDateString();
  };

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6 mb-8">
        {[...Array(6)].map((_, i) => (
          <Card key={i} className="sky-card animate-pulse">
            <div className="w-full h-48 bg-muted" />
            <CardContent className="p-4">
              <div className="h-4 bg-muted rounded mb-2" />
              <div className="h-3 bg-muted rounded mb-2" />
              <div className="h-3 bg-muted rounded" />
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  if (images.length === 0) {
    return (
      <div className="text-center py-12">
        <p className="text-muted-foreground mb-4">
          No astrophotography images found. Sync with Immich to get started.
        </p>
      </div>
    );
  }

  return (
    <>
      {/* Image Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6 mb-8">
        {images.map((image) => (
          <Card key={image.id} className="group cursor-pointer overflow-hidden" onClick={() => onImageClick(image)}>
            <div className="relative aspect-[4/3]">
              <RemoteImage
                src={imageUrl(image.id, 'thumbnail')}
                alt={image.title || "Astronomy image"}
                className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
              />
              <div className="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors duration-300" />
              <div className="absolute top-3 left-3 right-3 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity duration-300">
                <h3 className="text-sm font-medium text-white drop-shadow-lg line-clamp-2">
                  {image.title || "Untitled"}
                </h3>
              </div>
            </div>
            <CardContent className="p-4">
              <div className="flex justify-between items-center mb-2">
                <div className="text-xs text-muted-foreground">
                  {formatDate(image.captureDate)}
                </div>
                {getStatusBadge(image)}
              </div>
              <div className="text-xs text-muted-foreground space-y-1 font-mono mb-3">
                <div>{formatExposureData(image) || "No exposure data"}</div>
                <div>{formatIntegrationData(image) || "No integration data"}</div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Load More Button - Only show if there are more images to load */}
      {hasMoreImages && onLoadMore && (
        <div className="text-center mb-8">
          <Button 
            className="sky-button-primary"
            onClick={onLoadMore}
          >
            <Plus className="mr-2 h-4 w-4" />
            Load More Images ({visibleCount} of {totalImages})
          </Button>
        </div>
      )}

      {/* Equipment Section */}
      {equipment.length > 0 && (
        <div className="mb-8">
          <h2 className="text-xl font-bold text-foreground mb-4 flex items-center">
            <Settings className="mr-2 text-primary" />
            Equipment Gallery
          </h2>
          
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {equipment.map((item) => (
              <Card key={item.id} className="sky-card">
                {item.imageUrl && (
                  <img
                    src={item.imageUrl}
                    alt={item.name}
                    className="w-full h-32 object-cover"
                  />
                )}
                <CardContent className="p-3">
                  <h4 className="text-sm font-semibold text-foreground">{item.name}</h4>
                  <p className="text-xs text-muted-foreground">{item.type}</p>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      )}
    </>
  );
}
