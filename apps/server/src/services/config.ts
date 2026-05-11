import { storage } from './storage';

export interface AppConfig {
  immich: {
    host: string;
    apiKey: string;
    autoSync: boolean;
    syncFrequency: string;
    syncByAlbum: boolean;
    selectedAlbumIds: string[];
    metadataSyncEnabled: boolean;
    syncDescription: boolean;
    syncCoordinates: boolean;
    syncTags: boolean;
    immichMapping: string; // The path Immich uses internally (e.g., /usr/src/app/upload)
    localMapping: string;  // The path where that same volume is mounted in Sidereal (e.g., /immich-upload)
  };
  astrometry: {
    apiKey: string;
    enabled: boolean;
    autoEnabled: boolean;
    checkInterval: number;
    pollInterval: number;
    maxConcurrent: number;
    autoResubmit: boolean;
  };
  sidecar: {
    enabled: boolean;
    outputPath: string;
    organizeByDate: boolean;
  };
  app: {
    debugMode: boolean;
  };
}

class ConfigService {
  private config: AppConfig | null = null;

  async getConfig(): Promise<AppConfig> {
    if (this.config) {
      return this.config;
    }

    // Try to get admin settings first (from storage)
    const adminSettings = await this.getAdminSettings();
    
    // Fall back to hardcoded defaults if admin settings are not available
    const defaultConfig = this.getHardcodedDefaults();
    
    // Merge with admin settings taking priority over defaults
    this.config = {
      immich: {
        host: adminSettings.immich?.host || defaultConfig.immich.host,
        apiKey: adminSettings.immich?.apiKey || defaultConfig.immich.apiKey,
        autoSync: adminSettings.immich?.autoSync ?? defaultConfig.immich.autoSync,
        syncFrequency: adminSettings.immich?.syncFrequency || defaultConfig.immich.syncFrequency,
        syncByAlbum: adminSettings.immich?.syncByAlbum ?? defaultConfig.immich.syncByAlbum,
        selectedAlbumIds: adminSettings.immich?.selectedAlbumIds || defaultConfig.immich.selectedAlbumIds,
        metadataSyncEnabled: adminSettings.immich?.metadataSyncEnabled ?? defaultConfig.immich.metadataSyncEnabled,
        syncDescription: adminSettings.immich?.syncDescription ?? defaultConfig.immich.syncDescription,
        syncCoordinates: adminSettings.immich?.syncCoordinates ?? defaultConfig.immich.syncCoordinates,
        syncTags: adminSettings.immich?.syncTags ?? defaultConfig.immich.syncTags,
        immichMapping: process.env.IMMICH_MAPPING_PATH || adminSettings.immich?.immichMapping || defaultConfig.immich.immichMapping,
        localMapping: process.env.LOCAL_MAPPING_PATH || adminSettings.immich?.localMapping || defaultConfig.immich.localMapping,
      },
      astrometry: {
        apiKey: adminSettings.astrometry?.apiKey || defaultConfig.astrometry.apiKey,
        enabled: adminSettings.astrometry?.enabled ?? defaultConfig.astrometry.enabled,
        autoEnabled: adminSettings.astrometry?.autoEnabled ?? defaultConfig.astrometry.autoEnabled,
        checkInterval: adminSettings.astrometry?.checkInterval ?? defaultConfig.astrometry.checkInterval,
        pollInterval: adminSettings.astrometry?.pollInterval ?? defaultConfig.astrometry.pollInterval,
        maxConcurrent: adminSettings.astrometry?.maxConcurrent ?? defaultConfig.astrometry.maxConcurrent,
        autoResubmit: adminSettings.astrometry?.autoResubmit ?? defaultConfig.astrometry.autoResubmit,
      },
      sidecar: {
        enabled: adminSettings.sidecar?.enabled ?? defaultConfig.sidecar.enabled,
        outputPath: process.env.XMP_SIDECAR_PATH || adminSettings.sidecar?.outputPath || defaultConfig.sidecar.outputPath,
        organizeByDate: adminSettings.sidecar?.organizeByDate ?? defaultConfig.sidecar.organizeByDate,
      },
      app: {
        debugMode: adminSettings.app?.debugMode ?? defaultConfig.app.debugMode,
      },
    };

    return this.config;
  }

  private async getAdminSettings(): Promise<Partial<AppConfig>> {
    try {
      const settings = await storage.getAdminSettings();
      return settings;
    } catch (error) {
      console.warn('Failed to load admin settings:', error);
      return {};
    }
  }

  private getHardcodedDefaults(): AppConfig {
    // These are minimal defaults used only when DB is empty on first run
    // User must configure URLs/API keys through admin UI to make anything functional
    return {
      immich: {
        host: "",  // Empty - user must configure
        apiKey: "",  // Empty - user must configure
        autoSync: false,  // Disabled by default
        syncFrequency: "0 */4 * * *",  // Default cron (every 4 hours)
        syncByAlbum: false,  // Disabled by default
        selectedAlbumIds: [],  // Empty array
        metadataSyncEnabled: false,  // Disabled by default
        syncDescription: true,
        syncCoordinates: true,
        syncTags: true,
        immichMapping: '/usr/src/app/upload',
        localMapping: '/immich-upload',
      },
      astrometry: {
        apiKey: "",  // Empty - user must configure
        enabled: false,  // Disabled until user provides API key
        autoEnabled: false,  // Disabled by default
        checkInterval: 30,  // Reasonable default (seconds)
        pollInterval: 5,  // Reasonable default (seconds)
        maxConcurrent: 3,  // Reasonable default
        autoResubmit: false,  // Disabled by default
      },
      sidecar: {
        enabled: true,  // Generate XMP sidecars by default
        outputPath: './sidecars',  // Default path, overridden by XMP_SIDECAR_PATH env
        organizeByDate: true,  // Organize into yyyy-mm subdirectories
      },
      app: {
        debugMode: false,  // Disabled by default
      },
    };
  }

  async updateConfig(newConfig: Partial<AppConfig>): Promise<void> {
    // Update admin settings in storage
    await this.saveAdminSettings(newConfig);
    
    // Clear cached config so it will be reloaded
    this.config = null;
  }

  private async saveAdminSettings(config: Partial<AppConfig>): Promise<void> {
    try {
      await storage.updateAdminSettings(config);
    } catch (error) {
      console.error('Failed to save admin settings:', error);
      throw error;
    }
  }

  // Method to get specific config values
  async getImmichConfig() {
    const config = await this.getConfig();
    return config.immich;
  }

  async getAstrometryConfig() {
    const config = await this.getConfig();
    return config.astrometry;
  }

  async getSidecarConfig() {
    const config = await this.getConfig();
    return config.sidecar;
  }

  async getAppConfig() {
    const config = await this.getConfig();
    return config.app;
  }
}

export const configService = new ConfigService(); 