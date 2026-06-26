import type { ImageSourcePlugin } from '@shared/types';
import { localUploadSource } from './sources/local-upload.js';
import { urlUploadSource } from './sources/url-upload.js';
import { immichImageSource } from './sources/immich.js';

class SourceRegistry {
  private readonly plugins = new Map<string, ImageSourcePlugin>();

  register(plugin: ImageSourcePlugin): void {
    this.plugins.set(plugin.sourceType, plugin);
  }

  get(sourceType: string): ImageSourcePlugin | undefined {
    return this.plugins.get(sourceType);
  }

  list(): ImageSourcePlugin[] {
    return Array.from(this.plugins.values());
  }
}

export const sourceRegistry = new SourceRegistry();
sourceRegistry.register(localUploadSource);
sourceRegistry.register(urlUploadSource);
sourceRegistry.register(immichImageSource);
