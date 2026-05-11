import { existsSync, mkdirSync } from 'fs';
import { writeFile, unlink } from 'fs/promises';
import { dirname } from 'path';
import { configService } from './config';
import { filterRelevantTags } from './tags-utils';
import { AstrometryCalibration, AstrometryAnnotation } from './astrometry';
import type { Equipment } from '@shared/types';

export interface XmpSidecarData {
  calibration: AstrometryCalibration;
  annotations: AstrometryAnnotation[];
  machineTags: string[];
  imageId: string;
  filename: string;
  astrometryJobId: string;
  plateSolvedAt: Date;
  equipment?: Equipment[]; // Equipment used for this image
}

export class XmpSidecarService {
  /**
   * Generate XMP sidecar content for astronomical data
   */
  private generateXmpContent(data: XmpSidecarData): string {
    const {
      calibration,
      annotations,
      machineTags,
      imageId,
      filename,
      astrometryJobId,
      plateSolvedAt,
      equipment = []
    } = data;

    // Convert annotations to a more structured format
    const structuredAnnotations = annotations.map(ann => ({
      type: ann.type,
      names: ann.names,
      pixelX: ann.pixelx,
      pixelY: ann.pixely,
      radius: ann.radius,
      ra: ann.ra,
      dec: ann.dec,
      magnitude: ann.vmag
    }));

    // Group equipment by type for better organization
    const equipmentByType = equipment.reduce((acc, item) => {
      if (!acc[item.type]) {
        acc[item.type] = [];
      }
      acc[item.type].push(item);
      return acc;
    }, {} as Record<string, Equipment[]>);

    const xmpContent = `<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Sidereal">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    
    <!-- Dublin Core -->
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/">
      <dc:title>${filename}</dc:title>
      <dc:description>Astronomical image plate solved by Astrometry.net</dc:description>
      <dc:subject>
        <rdf:Bag>
          ${machineTags.map(tag => `<rdf:li>${this.escapeXml(tag)}</rdf:li>`).join('\n          ')}
        </rdf:Bag>
      </dc:subject>
    </rdf:Description>

    <!-- Custom Astronomical Metadata -->
    <rdf:Description rdf:about=""
      xmlns:astro="http://ns.astrometry.net/1.0/">
      <astro:plateSolved>true</astro:plateSolved>
      <astro:astrometryJobId>${astrometryJobId}</astro:astrometryJobId>
      <astro:plateSolvedAt>${plateSolvedAt.toISOString()}</astro:plateSolvedAt>
      <astro:imageId>${imageId}</astro:imageId>
      
      <!-- Calibration Data -->
      <astro:calibration>
        <rdf:Description>
          <astro:ra>${calibration.ra}</astro:ra>
          <astro:dec>${calibration.dec}</astro:dec>
          <astro:pixelScale>${calibration.pixscale}</astro:pixelScale>
          <astro:radius>${calibration.radius}</astro:radius>
          <astro:orientation>${calibration.orientation}</astro:orientation>
          <astro:parity>${calibration.parity}</astro:parity>
          ${calibration.width_arcsec ? `<astro:widthArcsec>${calibration.width_arcsec}</astro:widthArcsec>` : ''}
          ${calibration.height_arcsec ? `<astro:heightArcsec>${calibration.height_arcsec}</astro:heightArcsec>` : ''}
        </rdf:Description>
      </astro:calibration>

      <!-- Equipment Used -->
      ${Object.entries(equipmentByType).map(([type, items]) => `
      <astro:equipment>
        <rdf:Bag>
          ${items.map(item => `
          <rdf:li>
            <rdf:Description>
              <astro:type>${this.escapeXml(type)}</astro:type>
              <astro:name>${this.escapeXml(item.name)}</astro:name>
              ${item.description ? `<astro:description>${this.escapeXml(item.description)}</astro:description>` : ''}
              ${item.specifications ? `<astro:specifications>${this.escapeXml(JSON.stringify(item.specifications))}</astro:specifications>` : ''}
            </rdf:Description>
          </rdf:li>`).join('\n          ')}
        </rdf:Bag>
      </astro:equipment>`).join('\n      ')}

      <!-- Annotations -->
      <astro:annotations>
        <rdf:Bag>
          ${structuredAnnotations.map(ann => `
          <rdf:li>
            <rdf:Description>
              <astro:type>${ann.type}</astro:type>
              <astro:names>
                <rdf:Bag>
                  ${ann.names.map(name => `<rdf:li>${this.escapeXml(name)}</rdf:li>`).join('\n                  ')}
                </rdf:Bag>
              </astro:names>
              <astro:pixelX>${ann.pixelX}</astro:pixelX>
              <astro:pixelY>${ann.pixelY}</astro:pixelY>
              ${ann.radius ? `<astro:radius>${ann.radius}</astro:radius>` : ''}
              ${ann.ra ? `<astro:ra>${ann.ra}</astro:ra>` : ''}
              ${ann.dec ? `<astro:dec>${ann.dec}</astro:dec>` : ''}
              ${ann.magnitude ? `<astro:magnitude>${ann.magnitude}</astro:magnitude>` : ''}
            </rdf:Description>
          </rdf:li>`).join('\n          ')}
        </rdf:Bag>
      </astro:annotations>
    </rdf:Description>

    <!-- IPTC Keywords (for compatibility with photo management software) -->
    <rdf:Description rdf:about=""
      xmlns:iptc="http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/">
      <iptc:Keywords>
        <rdf:Bag>
          ${machineTags.map(tag => `<rdf:li>${this.escapeXml(tag)}</rdf:li>`).join('\n        ')}
        </rdf:Bag>
      </iptc:Keywords>
    </rdf:Description>

  </rdf:RDF>
</x:xmpmeta>`;

    return xmpContent;
  }

  /**
   * Escape XML special characters
   */
  private escapeXml(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&apos;');
  }

  /**
   * Write XMP sidecar file for an image
   */
  async writeSidecar(
    image: any,
    plateSolvingResult: any,
    astrometryJobId: string,
    equipment?: Equipment[],
    sidecarConfig?: { outputPath: string; organizeByDate: boolean },
  ): Promise<string> {
    try {
      const data: XmpSidecarData = {
        calibration: plateSolvingResult.calibration,
        annotations: plateSolvingResult.annotations,
        machineTags: filterRelevantTags(plateSolvingResult.machineTags),
        imageId: image.immichId,
        filename: image.filename,
        astrometryJobId,
        plateSolvedAt: new Date(),
        equipment
      };

      const xmpContent = this.generateXmpContent(data);

      // Determine the sidecar file path
      let sidecarDir: string;
      const config = await configService.getImmichConfig();

      if (image.originalPath && config.immichMapping && config.localMapping) {
        // Map the Immich internal path to our local mount point
        // Example: /usr/src/app/upload/library/image.jpg -> /immich-upload/library/image.jpg
        const relativePath = image.originalPath.startsWith(config.immichMapping)
          ? image.originalPath.substring(config.immichMapping.length)
          : image.originalPath;

        const localPath = `${config.localMapping}${relativePath}`;
        sidecarDir = dirname(localPath);
        console.log(`Mapped Immich path ${image.originalPath} to local path ${localPath}`);
      } else {
        // Fallback to the configured XMP_SIDECAR_PATH
        const baseDir = sidecarConfig?.outputPath || process.env.XMP_SIDECAR_PATH || './sidecars';
        sidecarDir = baseDir;

        // Organize by capture date if enabled
        if (sidecarConfig?.organizeByDate && image.captureDate) {
          const date = new Date(image.captureDate);
          const yyyy = date.getFullYear();
          const mm = String(date.getMonth() + 1).padStart(2, '0');
          sidecarDir = `${baseDir}/${yyyy}-${mm}`;
        }
      }

      const sidecarPath = `${sidecarDir}/${image.filename}.xmp`;

      console.log(`Attempting to write XMP sidecar to: ${sidecarPath}`);

      // Ensure directory exists with proper error handling
      try {
        if (!existsSync(sidecarDir)) {
          console.log(`Creating sidecar directory: ${sidecarDir}`);
          mkdirSync(sidecarDir, { recursive: true });
        }
      } catch (dirError) {
        console.error(`Failed to create sidecar directory ${sidecarDir}:`, dirError);
        throw new Error(`Cannot create sidecar directory: ${dirError}`);
      }

      // Check if directory is writable
      try {
        const testFile = `${sidecarDir}/.test-write`;
        await writeFile(testFile, 'test', 'utf8');
        await unlink(testFile);
      } catch (writeError) {
        console.error(`Directory ${sidecarDir} is not writable:`, writeError);
        throw new Error(`Sidecar directory is not writable: ${writeError}`);
      }

      // Write the XMP file
      await writeFile(sidecarPath, xmpContent, 'utf8');
      
      console.log(`XMP sidecar written successfully: ${sidecarPath}`);
      return sidecarPath;
    } catch (error) {
      console.error(`Failed to write XMP sidecar for image ${image.id}:`, error);
      throw error;
    }
  }

  /**
   * Resolve the sidecar file path for an image, checking both date-organized and flat layouts
   */
  async resolveSidecarPath(
    image: any,
    sidecarConfig?: { outputPath: string; organizeByDate: boolean },
  ): Promise<string | null> {
    const config = await configService.getImmichConfig();

    // Check mapped path first (where Immich stores its files)
    if (image.originalPath && config.immichMapping && config.localMapping) {
      const relativePath = image.originalPath.startsWith(config.immichMapping)
        ? image.originalPath.substring(config.immichMapping.length)
        : image.originalPath;

      const mappedPath = `${config.localMapping}${relativePath}.xmp`;
      if (existsSync(mappedPath)) return mappedPath;
    }

    const baseDir = sidecarConfig?.outputPath || process.env.XMP_SIDECAR_PATH || './sidecars';

    // Check date-organized path first
    if (image.captureDate) {
      const date = new Date(image.captureDate);
      const yyyy = date.getFullYear();
      const mm = String(date.getMonth() + 1).padStart(2, '0');
      const datePath = `${baseDir}/${yyyy}-${mm}/${image.filename}.xmp`;
      if (existsSync(datePath)) return datePath;
    }

    // Fall back to flat path
    const flatPath = `${baseDir}/${image.filename}.xmp`;
    if (existsSync(flatPath)) return flatPath;

    return null;
  }

  /**
   * Generate a human-readable summary of the astronomical data
   */
  generateSummary(data: XmpSidecarData): string {
    const { calibration, annotations, machineTags, equipment = [] } = data;
    
    let summary = `Astronomical Image Analysis Summary
=====================================

Plate Solving Results:
- RA: ${calibration.ra.toFixed(6)}°
- Dec: ${calibration.dec.toFixed(6)}°
- Pixel Scale: ${calibration.pixscale.toFixed(3)} arcsec/pixel
- Field of View: ${(calibration.radius * 2).toFixed(1)} arcmin
- Rotation: ${calibration.orientation.toFixed(1)}°
- Parity: ${calibration.parity}

Equipment Used:
${equipment.map(item => `- ${item.type.toUpperCase()}: ${item.name}${item.description ? ` (${item.description})` : ''}`).join('\n')}

Identified Objects (${annotations.length}):
${annotations.map(ann => `- ${ann.type.toUpperCase()}: ${ann.names.join(', ')}`).join('\n')}

Machine Tags:
${machineTags.map(tag => `- ${tag}`).join('\n')}

Generated by Sidereal using Astrometry.net
Job ID: ${data.astrometryJobId}
Date: ${data.plateSolvedAt.toISOString()}`;

    return summary;
  }
}

export const xmpSidecarService = new XmpSidecarService(); 