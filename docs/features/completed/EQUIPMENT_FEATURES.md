# Equipment Management Features

This document describes the equipment management functionality added to Sidereal, which allows users to track and manage equipment used for astrophotography images.

## Overview

The equipment management system provides:

1. **Equipment Catalog**: Store and manage your astrophotography equipment
2. **Image-Equipment Relationships**: Link specific equipment to individual images
3. **Equipment Settings**: Track how equipment was configured for each image
4. **XMP Sidecar Integration**: Equipment information is embedded in XMP sidecar files
5. **Visual Equipment Display**: Equipment details are shown in the image overlay

## Database Schema

### Equipment Table
```sql
equipment (
  id: serial primary key
  name: text not null
  type: text not null (telescope, camera, mount, filter, accessories)
  specifications: json
  imageUrl: text
  description: text
  createdAt: timestamp
  updatedAt: timestamp
)
```

### Image Equipment Junction Table
```sql
image_equipment (
  id: serial primary key
  imageId: integer references astrophotography_images(id)
  equipmentId: integer references equipment(id)
  settings: json (equipment-specific settings for this image)
  notes: text (additional notes about equipment usage)
  createdAt: timestamp
)
```

## Features

### 1. Equipment Catalog

- **Add Equipment**: Create equipment entries with name, type, specifications, and description
- **Equipment Types**: Support for telescope, camera, mount, filter, and accessories
- **Specifications**: JSON field for storing detailed equipment specifications
- **Equipment Images**: Optional image URLs for equipment visualization

### 2. Image-Equipment Relationships

- **Assign Equipment**: Link equipment to specific images
- **Equipment Settings**: Store how equipment was configured for each image (e.g., focal length, aperture, gain settings)
- **Usage Notes**: Add notes about how equipment was used
- **Multiple Equipment**: One image can use multiple pieces of equipment

### 3. Equipment Display

- **Equipment Section**: Dedicated section in the image overlay showing equipment used
- **Grouped by Type**: Equipment is organized by type (telescope, camera, mount, etc.)
- **Settings Display**: Shows equipment-specific settings used for the image
- **Visual Icons**: Different icons for different equipment types

### 4. Equipment Management Interface

- **Edit Equipment**: Modal interface for managing equipment assignments
- **Add/Remove Equipment**: Easy addition and removal of equipment from images
- **Settings Management**: Edit equipment settings and notes for each image
- **Equipment Selection**: Dropdown to select from available equipment

### 5. XMP Sidecar Integration

Equipment information is automatically included in XMP sidecar files when images are plate-solved:

```xml
<astro:equipment>
  <rdf:Bag>
    <rdf:li>
      <rdf:Description>
        <astro:type>telescope</astro:type>
        <astro:name>William Optics RedCat 51</astro:name>
        <astro:description>Compact apochromatic refractor</astro:description>
        <astro:specifications>{"aperture": "51mm", "focalLength": "250mm"}</astro:specifications>
      </rdf:Description>
    </rdf:li>
  </rdf:Bag>
</astro:equipment>
```

## API Endpoints

### Equipment Management
- `GET /api/equipment` - Get all equipment
- `POST /api/equipment` - Create new equipment

### Image Equipment Relationships
- `GET /api/images/:id/equipment` - Get equipment for a specific image
- `POST /api/images/:id/equipment` - Add equipment to an image
- `DELETE /api/images/:imageId/equipment/:equipmentId` - Remove equipment from an image
- `PUT /api/images/:imageId/equipment/:equipmentId` - Update equipment settings for an image

## Usage Examples

### Adding Equipment to an Image

1. Open an image in the overlay view
2. Click the "Edit" button in the Equipment section
3. Select equipment from the dropdown
4. Add optional settings (e.g., focal length, aperture)
5. Add optional notes about usage
6. Click "Add Equipment"

### Managing Equipment Settings

1. In the equipment manager, click the edit icon on any equipment
2. Modify settings like focal length, aperture, gain, etc.
3. Update notes about how the equipment was used
4. Save changes

### Viewing Equipment Information

Equipment information is automatically displayed in the image overlay:
- Equipment is grouped by type
- Shows equipment name, description, and specifications
- Displays image-specific settings and notes
- Uses appropriate icons for different equipment types

## Data Migration

The system includes backward compatibility with existing image data. Legacy equipment fields (telescope, camera, mount, filters) are preserved and can be migrated to the new equipment relationship system.

## Future Enhancements

- Equipment usage statistics and analytics
- Equipment maintenance tracking
- Equipment cost tracking
- Equipment sharing between users
- Equipment recommendations based on image targets
- Integration with equipment databases and APIs 