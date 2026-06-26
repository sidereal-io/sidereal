import { useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { Header } from "@/components/header";
import { SearchFilters } from "@/components/search-filters";
import { ImageGallery } from "@/components/image-gallery";
import { Sidebar } from "@/components/sidebar";
import { ImageOverlay } from "@/components/image-overlay";
import { usePlateSolvingUpdates, useSourceSyncUpdates } from "@/hooks/use-socket";
import { useSearch, useLocation } from "wouter";
import type { AstroImage, Equipment } from "@shared/schema";

export default function Home() {
  const [selectedImage, setSelectedImage] = useState<AstroImage | null>(null);
  const searchString = useSearch();
  const [, setLocation] = useLocation();

  const [filters, setFilters] = useState({
    objectType: "",
    tags: [] as string[],
    plateSolved: undefined as boolean | undefined,
    constellation: "",
    search: "",
    equipmentId: undefined as number | undefined,
    equipmentName: undefined as string | undefined,
  });
  const [visibleCount, setVisibleCount] = useState(12); // Show 12 images initially

  type Stats = { totalImages: number; plateSolved: number; totalHours: number; uniqueTargets: number; };
  type Tag = { tag: string; count: number };

  const { data: images = [], isLoading: imagesLoading, refetch: refetchImages } = useQuery<AstroImage[]>({
    queryKey: ["/api/images", { objectType: filters.objectType, tags: filters.tags, plateSolved: filters.plateSolved, constellation: filters.constellation, equipmentId: filters.equipmentId }],
    enabled: true,
  });

  // Handle deep-linking to a specific image via ?image=ID
  useEffect(() => {
    if (images.length > 0 && searchString) {
      const params = new URLSearchParams(searchString);
      const imageId = params.get("image");
      if (imageId) {
        const image = images.find(img => img.id.toString() === imageId);
        if (image) {
          setSelectedImage(image);
        }
      }
    }
  }, [images, searchString]);

  const { data: stats, refetch: refetchStats } = useQuery<Stats>({
    queryKey: ["/api/stats"],
    enabled: true,
  });

  const { data: equipment = [] } = useQuery<Equipment[]>({
    queryKey: ["/api/equipment"],
    enabled: true,
  });

  const { data: tags = [] } = useQuery<Tag[]>({
    queryKey: ["/api/tags"],
    enabled: true,
  });

  // Listen for real-time plate solving updates
  usePlateSolvingUpdates((update) => {
    console.log('Received plate solving update:', update);
    // Refresh data when plate solving status changes
    refetchImages();
    refetchStats();
  });

  // Listen for real-time source sync updates
  useSourceSyncUpdates((update) => {
    console.log('Received source sync update:', update);
    // Refresh data when source sync completes
    refetchImages();
    refetchStats();
  });

  const filteredImages = images.filter((image: AstroImage) => {
    if (filters.search && !image.title.toLowerCase().includes(filters.search.toLowerCase())) {
      return false;
    }
    // Advanced filters (client-side)
    const f = filters as any;
    if (f.dateFrom && image.captureDate) {
      if (new Date(image.captureDate) < new Date(f.dateFrom)) return false;
    }
    if (f.dateTo && image.captureDate) {
      if (new Date(image.captureDate) > new Date(f.dateTo + "T23:59:59")) return false;
    }
    if (f.minIntegration !== undefined && (image.totalIntegration || 0) < f.minIntegration) {
      return false;
    }
    return true;
  });

  // Reset visible count when filters change
  const handleFiltersChange = (newFilters: typeof filters) => {
    setFilters(newFilters);
    setVisibleCount(12); // Reset to initial count
  };

  const handleLoadMore = () => {
    setVisibleCount(prev => prev + 12); // Load 12 more images
  };

  // Get only the visible images based on pagination
  const visibleImages = filteredImages.slice(0, visibleCount);
  const hasMoreImages = visibleCount < filteredImages.length;

  const handleCloseOverlay = () => {
    setSelectedImage(null);
    // Remove the image parameter from the URL when overlay is closed
    if (searchString.includes("image=")) {
      const params = new URLSearchParams(searchString);
      params.delete("image");
      const newSearch = params.toString();
      // Ensure we navigate back cleanly
      setLocation(newSearch ? `/?${newSearch}` : "/", { replace: true });
    }
  };

  return (
    <div className="min-h-screen bg-background overflow-x-hidden w-full">
      <Header />
      <SearchFilters
        filters={filters}
        onFiltersChange={handleFiltersChange}
        stats={stats}
        totalCount={images.length}
        filteredCount={filteredImages.length}
      />
      
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6 w-full overflow-x-hidden">
        <div className="flex flex-col lg:flex-row gap-6">
          <main className="flex-1">
            <ImageGallery
              images={visibleImages}
              equipment={equipment}
              onImageClick={(image) => {
                setSelectedImage(image);
                // Also update URL when clicking from gallery for bookmarkability
                setLocation(`/?image=${image.id}`, { replace: true });
              }}
              isLoading={imagesLoading}
              hasMoreImages={hasMoreImages}
              onLoadMore={handleLoadMore}
              totalImages={filteredImages.length}
              visibleCount={visibleCount}
            />
          </main>
          
          <Sidebar 
            stats={stats}
            tags={tags}
            onTagClick={(tag) => {
              setFilters(prev => ({
                ...prev,
                tags: prev.tags.includes(tag) 
                  ? prev.tags.filter(t => t !== tag)
                  : [...prev.tags, tag]
              }));
            }}
          />
        </div>
      </div>

      {selectedImage && (
        <ImageOverlay
          image={selectedImage}
          onClose={handleCloseOverlay}
          onFilterByEquipment={(equipmentId, equipmentName) => {
            setFilters(prev => ({ ...prev, equipmentId, equipmentName }));
            setSelectedImage(null);
          }}
        />
      )}
    </div>
  );
}
