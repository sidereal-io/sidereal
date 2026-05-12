# Sidereal API Documentation

This document describes the REST API endpoints available in Sidereal.

## Base URL

All API endpoints are relative to your Sidereal server's base URL:
```
http://localhost:5000/api
```

## Authentication

Currently, Sidereal does not require authentication for API access. All endpoints are publicly accessible when the server is running.

## Response Format

All API responses return JSON with the following general structure:

**Success Response:**
```json
{
  "data": {...},
  "message": "Operation successful"
}
```

**Error Response:**
```json
{
  "message": "Error description",
  "error": "Additional error details"
}
```

## Endpoints

### Images

#### Get All Images
```
GET /api/images
```

**Query Parameters:**
- `objectType` (string) - Filter by object type
- `tags` (string|array) - Filter by tags
- `plateSolved` (boolean) - Filter by plate solving status
- `constellation` (string) - Filter by constellation

**Response:**
```json
[
  {
    "id": 1,
    "immichId": "uuid",
    "title": "M31 Andromeda Galaxy",
    "filename": "m31.jpg",
    "thumbnailUrl": "/api/assets/uuid/thumbnail",
    "fullUrl": "/api/assets/uuid/thumbnail?size=preview",
    "captureDate": "2023-10-15T22:30:00Z",
    "focalLength": 600,
    "aperture": "f/5.6",
    "iso": 1600,
    "exposureTime": "300s",
    "plateSolved": true,
    "ra": "0.712",
    "dec": "41.269",
    "constellation": "Andromeda",
    "tags": ["galaxy", "deep-sky"],
    "objectType": "Galaxy"
  }
]
```

#### Get Single Image
```
GET /api/images/{id}
```

**Parameters:**
- `id` (integer) - Image ID

**Response:** Single image object (same structure as above)

#### Update Image
```
PATCH /api/images/{id}
```

**Parameters:**
- `id` (integer) - Image ID

**Request Body:**
```json
{
  "title": "Updated title",
  "objectType": "Galaxy",
  "tags": ["galaxy", "messier"],
  "description": "Updated description"
}
```

**Response:** Updated image object

### Plate Solving

#### Submit Image for Plate Solving
```
POST /api/images/{id}/plate-solve
```

**Parameters:**
- `id` (integer) - Image ID

**Response:**
```json
{
  "message": "Image plate solving completed successfully",
  "result": {
    "calibration": {
      "ra": 10.684,
      "dec": 41.269,
      "pixscale": 2.77,
      "radius": 0.5,
      "orientation": 90.0,
      "parity": 1
    },
    "annotations": [
      {
        "type": "star",
        "names": ["HD 12345"],
        "ra": 10.684,
        "dec": 41.269,
        "pixelx": 1024,
        "pixely": 768,
        "radius": 10
      }
    ],
    "machineTags": ["galaxy", "andromeda", "messier"]
  }
}
```

#### Get Plate Solving Jobs
```
GET /api/plate-solving/jobs
```

**Response:**
```json
[
  {
    "id": 1,
    "imageId": 1,
    "astrometrySubmissionId": "12345",
    "astrometryJobId": "67890",
    "status": "success",
    "submittedAt": "2023-10-15T22:30:00Z",
    "completedAt": "2023-10-15T22:35:00Z",
    "result": {
      "ra": 10.684,
      "dec": 41.269,
      "pixscale": 2.77
    }
  }
]
```

#### Bulk Submit for Plate Solving
```
POST /api/plate-solving/bulk
```

**Request Body:**
```json
{
  "imageIds": [1, 2, 3, 4, 5]
}
```

**Response:**
```json
{
  "message": "Bulk plate solving submission completed",
  "results": [
    {
      "imageId": 1,
      "success": true,
      "submissionId": "12345",
      "jobId": 1,
      "message": "Submitted for plate solving"
    },
    {
      "imageId": 2,
      "success": false,
      "error": "Image already plate solved"
    }
  ]
}
```

#### Get Image Annotations
```
GET /api/images/{id}/annotations
```

**Parameters:**
- `id` (integer) - Image ID

**Response:**
```json
{
  "annotations": [
    {
      "type": "star",
      "names": ["HD 12345"],
      "ra": 10.684,
      "dec": 41.269,
      "pixelx": 1024,
      "pixely": 768,
      "radius": 10
    }
  ],
  "calibration": {
    "ra": 10.684,
    "dec": 41.269,
    "pixscale": 2.77,
    "radius": 0.5,
    "orientation": 90.0
  },
  "imageDimensions": {
    "width": 2048,
    "height": 1536
  }
}
```

### Equipment

#### Get All Equipment
```
GET /api/equipment
```

**Response:**
```json
[
  {
    "id": 1,
    "name": "Celestron EdgeHD 11",
    "type": "telescope",
    "specifications": {
      "aperture": "279mm",
      "focalLength": "2800mm",
      "focalRatio": "f/10"
    },
    "description": "Schmidt-Cassegrain telescope",
    "createdAt": "2023-10-15T22:30:00Z",
    "updatedAt": "2023-10-15T22:30:00Z"
  }
]
```

#### Create Equipment
```
POST /api/equipment
```

**Request Body:**
```json
{
  "name": "Canon EOS R5",
  "type": "camera",
  "specifications": {
    "sensor": "Full Frame",
    "resolution": "45MP",
    "pixelSize": "4.39μm"
  },
  "description": "Mirrorless camera for astrophotography"
}
```

**Response:** Created equipment object

#### Update Equipment
```
PUT /api/equipment/{id}
```

**Parameters:**
- `id` (integer) - Equipment ID

**Request Body:** Same as create equipment

**Response:** Updated equipment object

#### Delete Equipment
```
DELETE /api/equipment/{id}
```

**Parameters:**
- `id` (integer) - Equipment ID

**Response:**
```json
{
  "message": "Equipment deleted successfully"
}
```

#### Get Equipment for Image
```
GET /api/images/{id}/equipment
```

**Parameters:**
- `id` (integer) - Image ID

**Response:**
```json
[
  {
    "id": 1,
    "name": "Celestron EdgeHD 11",
    "type": "telescope",
    "specifications": {...},
    "settings": {
      "focalLength": 2800,
      "focalRatio": "f/10"
    },
    "notes": "Used with field flattener"
  }
]
```

#### Add Equipment to Image
```
POST /api/images/{id}/equipment
```

**Parameters:**
- `id` (integer) - Image ID

**Request Body:**
```json
{
  "equipmentId": 1,
  "settings": {
    "focalLength": 2800,
    "focalRatio": "f/10"
  },
  "notes": "Used with field flattener"
}
```

**Response:** Image-equipment relationship object

#### Remove Equipment from Image
```
DELETE /api/images/{imageId}/equipment/{equipmentId}
```

**Parameters:**
- `imageId` (integer) - Image ID
- `equipmentId` (integer) - Equipment ID

**Response:**
```json
{
  "message": "Equipment removed from image"
}
```

### Admin & Configuration

#### Get Admin Settings
```
GET /api/admin/settings
```

**Response:**
```json
{
  "immich": {
    "host": "http://localhost:2283",
    "apiKey": "your-api-key",
    "autoSync": true,
    "syncFrequency": "0 */4 * * *",
    "syncByAlbum": true,
    "selectedAlbumIds": ["uuid1", "uuid2"]
  },
  "astrometry": {
    "apiKey": "your-api-key",
    "enabled": true,
    "checkInterval": 30,
    "pollInterval": 5,
    "maxConcurrent": 3,
    "autoResubmit": false
  },
  "app": {
    "debugMode": false
  }
}
```

#### Update Admin Settings
```
POST /api/admin/settings
```

**Request Body:** Same structure as GET response

**Response:**
```json
{
  "success": true,
  "message": "Settings saved successfully"
}
```

#### Test Immich Connection
```
POST /api/test-immich-connection
```

**Request Body:**
```json
{
  "host": "http://localhost:2283",
  "apiKey": "your-api-key"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Connection successful!"
}
```

#### Test Astrometry Connection
```
POST /api/test-astrometry-connection
```

**Request Body:**
```json
{
  "apiKey": "your-api-key"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Connection successful!"
}
```

### Sync & Integration

> **Note:** These endpoints are also available under `/api/immich/*`. See the [Immich Integration](#immich-integration) section for the full set of Immich-related endpoints including metadata sync.

### Statistics & Data

#### Get Statistics
```
GET /api/stats
```

**Response:**
```json
{
  "totalImages": 150,
  "plateSolvedImages": 120,
  "totalEquipment": 15,
  "totalIntegrationHours": 45.7,
  "objectTypeCounts": {
    "Galaxy": 45,
    "Nebula": 38,
    "Star Cluster": 25,
    "Planet": 12
  },
  "plateSolvingStats": {
    "total": 125,
    "pending": 5,
    "successful": 120,
    "failed": 0
  }
}
```

#### Get Tags
```
GET /api/tags
```

**Response:**
```json
[
  {
    "tag": "galaxy",
    "count": 45
  },
  {
    "tag": "nebula",
    "count": 38
  },
  {
    "tag": "messier",
    "count": 25
  }
]
```

#### Get Constellations
```
GET /api/constellations
```

**Response:**
```json
[
  "Andromeda",
  "Cassiopeia",
  "Cygnus",
  "Orion",
  "Ursa Major"
]
```

### Notifications

#### Get Notifications
```
GET /api/notifications
```

**Response:**
```json
[
  {
    "id": 1,
    "type": "info",
    "title": "Plate Solving Complete",
    "message": "Successfully plate solved 5 images",
    "details": {
      "imageCount": 5,
      "duration": "2 minutes"
    },
    "acknowledged": false,
    "createdAt": "2023-10-15T22:30:00Z"
  }
]
```

#### Acknowledge Notification
```
POST /api/notifications/{id}/acknowledge
```

**Parameters:**
- `id` (integer) - Notification ID

**Response:**
```json
{
  "success": true,
  "message": "Notification acknowledged"
}
```

### Health & Monitoring

#### Health Check
```
GET /api/health
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2023-10-15T22:30:00Z",
  "uptime": 86400,
  "database": "healthy",
  "worker": {
    "enabled": true,
    "running": true,
    "pid": 1234,
    "restartAttempts": 0
  },
  "version": "1.2.0",
  "nodeVersion": "v20.10.0"
}
```

### Asset Proxy

#### Get Asset
```
GET /api/assets/{assetId}/{type}
```

**Parameters:**
- `assetId` (string) - Immich asset ID
- `type` (string) - Asset type (`thumbnail`, `original`, etc.)

**Query Parameters:**
- `size` (string) - Size parameter (e.g., `preview`)

**Response:** Binary image data

### Catalog

#### Browse Catalog
```
GET /api/catalog/browse
```

**Query Parameters:**
- `page` (integer) - Page number (default: 1)
- `limit` (integer) - Items per page (default: 50)
- `q` (string) - Search query
- `type` (string) - Filter by object type
- `constellation` (string) - Filter by constellation
- `minMag` / `maxMag` (number) - Magnitude range
- `minSize` (number) - Minimum angular size (arcmin)
- `messierOnly` (boolean) - Show only Messier objects
- `sortBy` (string) - Sort field: `name`, `vMag`, `majorAxis`, `bestNow`
- `sortOrder` (string) - `asc` or `desc`
- `latitude` (number) - Observer latitude for visibility filtering
- `hideBelow` (boolean) - Hide objects below the horizon
- `names` (string) - Comma-separated list of object names

**Response:**
```json
{
  "items": [
    {
      "name": "NGC 224",
      "type": "Galaxy",
      "constellation": "And",
      "raDeg": 10.684,
      "decDeg": 41.269,
      "vMag": 3.44,
      "majorAxis": 190.5
    }
  ],
  "total": 13957,
  "page": 1,
  "limit": 50
}
```

#### Search Catalog (Autocomplete)
```
GET /api/catalog/search
```

**Query Parameters:**
- `q` (string, required) - Search query (min 1 character)

**Response:** Array of matching catalog objects (max 20)

#### Get Catalog Status
```
GET /api/catalog/status
```

**Response:**
```json
{
  "objectCount": 13957,
  "lastUpdated": "2024-01-15T10:00:00Z",
  "commitSha": "abc123"
}
```

#### Get Catalog Object
```
GET /api/catalog/{name}
```

**Parameters:**
- `name` (string) - Object name (URL-encoded, e.g., `NGC%20224`)

**Response:** Single catalog object or `404`

#### Get DSS Thumbnail
```
GET /api/catalog/thumbnail/{name}
```

Fetches a Digitized Sky Survey preview image for the named object. Results are cached on disk; subsequent requests are served as static files.

**Parameters:**
- `name` (string) - Object name (URL-encoded)

**Response:** JPEG image (`image/jpeg`) with a one-year cache header, or `404` if the object has no coordinates.

#### Load/Reload Catalog
```
POST /api/catalog/load
```

Loads or reloads the OpenNGC catalog from GitHub.

**Response:**
```json
{
  "message": "Loaded 13957 catalog objects",
  "count": 13957
}
```

#### Check for Catalog Updates
```
POST /api/catalog/check-updates
```

**Response:** Update availability status

#### Backfill Target Names
```
POST /api/catalog/backfill-targets
```

Re-matches all plate-solved images that have tags but no `targetName` against the catalog.

**Response:**
```json
{
  "message": "Backfill complete: 12 images matched, 45 already had targets",
  "matched": 12,
  "skipped": 45,
  "total": 150
}
```

---

### User Targets

Manage personal annotations (notes, tags) on catalog objects.

#### Get All User Targets
```
GET /api/user-targets
```

**Response:** Array of user target annotations

#### Get Single User Target
```
GET /api/user-targets/{catalogName}
```

**Parameters:**
- `catalogName` (string) - Catalog object name (URL-encoded)

**Response:** User target annotation or `404`

#### Create/Update User Target
```
PUT /api/user-targets/{catalogName}
```

**Parameters:**
- `catalogName` (string) - Catalog object name (URL-encoded)

**Request Body:**
```json
{
  "notes": "Good early autumn target, needs 6+ hours integration",
  "tags": ["wishlist", "autumn"]
}
```

**Response:** Created or updated user target annotation

#### Delete User Target
```
DELETE /api/user-targets/{catalogName}
```

**Response:**
```json
{
  "message": "User target deleted"
}
```

---

### Immich Integration

#### Sync Images from Immich
```
POST /api/immich/sync-immich
```

**Response:**
```json
{
  "message": "Successfully synced 25 new images from Immich. Removed 2 images no longer in Immich.",
  "syncedCount": 25,
  "removedCount": 2
}
```

#### Test Immich Connection
```
POST /api/immich/test-immich-connection
```

**Request Body:**
```json
{
  "host": "http://localhost:2283",
  "apiKey": "your-api-key"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Connection successful!"
}
```

#### Get Immich Albums
```
POST /api/immich/albums
```

**Request Body:**
```json
{
  "host": "http://localhost:2283",
  "apiKey": "your-api-key"
}
```

**Response:**
```json
[
  { "id": "uuid1", "albumName": "Astrophotography" },
  { "id": "uuid2", "albumName": "Deep Sky Objects" }
]
```

#### Sync Metadata to Immich (Single Image)
```
POST /api/immich/sync-metadata/{imageId}
```

**Parameters:**
- `imageId` (integer) - Image ID

**Response:**
```json
{
  "message": "Metadata synced to Immich"
}
```

#### Sync Metadata to Immich (All Images)
```
POST /api/immich/sync-metadata-all
```

**Response:**
```json
{
  "message": "Synced 120 images, 2 failed",
  "synced": 120,
  "failed": 2
}
```

---

### Locations

Manage saved observing sites. Coordinates applied to images via the location editor also sync back to the corresponding Immich asset.

#### Get All Locations
```
GET /api/locations
```

**Response:** Array of saved locations

#### Get Single Location
```
GET /api/locations/{id}
```

**Parameters:**
- `id` (integer) - Location ID

**Response:** Location object or `404`

#### Create Location
```
POST /api/locations
```

**Request Body:**
```json
{
  "name": "Bortle 3 Site",
  "latitude": 34.05,
  "longitude": -118.25,
  "altitude": 1200
}
```

**Response:** Created location object

#### Update Location
```
PATCH /api/locations/{id}
```

**Request Body:** Partial location fields to update

**Response:** Updated location object or `404`

#### Delete Location
```
DELETE /api/locations/{id}
```

**Response:**
```json
{
  "message": "Location deleted"
}
```

---

### Equipment Groups

Bundle equipment into reusable presets that can be applied to images.

#### Get All Equipment Groups
```
GET /api/equipment-groups
```

**Response:** Array of equipment groups with their members

#### Get Single Equipment Group
```
GET /api/equipment-groups/{id}
```

**Parameters:**
- `id` (integer) - Group ID

**Response:** Equipment group with members or `404`

#### Create Equipment Group
```
POST /api/equipment-groups
```

**Request Body:**
```json
{
  "name": "Widefield Rig",
  "description": "Rokinon 135mm + ASI2600MC",
  "memberIds": [1, 3, 7]
}
```

**Response:** Created group with members

#### Update Equipment Group
```
PUT /api/equipment-groups/{id}
```

**Request Body:**
```json
{
  "name": "Updated Name",
  "description": "Updated description"
}
```

**Response:** Updated group with members or `404`

#### Delete Equipment Group
```
DELETE /api/equipment-groups/{id}
```

**Response:**
```json
{
  "message": "Equipment group deleted successfully"
}
```

#### Replace Group Members
```
PUT /api/equipment-groups/{id}/members
```

**Request Body:**
```json
{
  "memberIds": [1, 3, 7, 12]
}
```

**Response:** Updated group with members

#### Apply Group to Image
```
POST /api/equipment-groups/{id}/apply/{imageId}
```

Assigns all equipment in the group to the specified image.

**Parameters:**
- `id` (integer) - Group ID
- `imageId` (integer) - Image ID

**Response:**
```json
{
  "added": [...],
  "message": "4 equipment item(s) assigned"
}
```

---

### Sky Map

#### Get Sky Map Markers
```
GET /api/sky-map/markers
```

Returns plate-solved images as sky map markers with position and metadata.

**Response:**
```json
[
  {
    "id": 1,
    "title": "M31 Andromeda Galaxy",
    "ra": "0.712",
    "dec": "41.269",
    "thumbnailUrl": "/api/assets/uuid/thumbnail",
    "objectType": "Galaxy",
    "constellation": "Andromeda",
    "fieldOfView": 1.5
  }
]
```

---

### Targets

Aggregate view of imaged targets grouped by object name.

#### List Targets
```
GET /api/targets
```

**Query Parameters:**
- `search` (string) - Filter by target name or common names
- `type` (string) - Filter by object type
- `constellation` (string) - Filter by constellation

**Response:** Array of target summaries (name, image count, total integration time, etc.)

#### Get Single Target
```
GET /api/targets/{name}
```

**Parameters:**
- `name` (string) - Target name (URL-encoded)

**Response:** Target detail with image list or `404`

---

### Database (Admin)

#### Get Database Info
```
GET /api/admin/database
```

**Response:**
```json
{
  "type": "sqlite",
  "path": "/app/config/sidereal.db",
  "sizeBytes": 1048576,
  "lastModified": "2024-01-15T10:00:00Z"
}
```

#### Download Database Backup
```
GET /api/admin/database/backup
```

Downloads the SQLite database file as an attachment. Only available for SQLite databases; returns `400` for PostgreSQL.

**Response:** Binary SQLite file (`application/x-sqlite3`) with `Content-Disposition` header.

---

### Cron Jobs (Admin)

#### Get Cron Job Status
```
GET /api/admin/cron-jobs
```

Returns the status of all scheduled background jobs (Immich sync, notification cleanup).

**Response:** Array of cron job objects with name, schedule, last run time, and last error.

---

## Error Handling

The API uses standard HTTP status codes:

- `200` - Success
- `400` - Bad Request (missing or invalid parameters)
- `404` - Not Found (resource doesn't exist)
- `500` - Internal Server Error

All error responses include a descriptive message:

```json
{
  "message": "Image not found",
  "error": "Additional error details"
}
```

## Rate Limiting

Currently, there are no rate limits implemented. However, be mindful of:
- Bulk operations (like plate solving) can be resource-intensive
- Image proxy requests are forwarded to Immich
- Astrometry.net API calls are subject to their rate limits

## WebSocket Events

Sidereal uses native WebSocket for real-time updates. Connect to `ws://<host>/ws` and listen for JSON messages:

### Plate Solving Updates
```javascript
const ws = new WebSocket(`ws://${location.host}/ws`);
ws.onmessage = (event) => {
  const { event: type, data } = JSON.parse(event.data);
  if (type === 'plate-solving-update') {
    console.log('Plate solving update:', data);
    // data: { jobId, status, result, imageId, message }
  }
};
```

## SDK Usage

For TypeScript/JavaScript applications, you can use the API with fetch:

```javascript
// Get all images
const response = await fetch('/api/images');
const images = await response.json();

// Submit for plate solving
const result = await fetch(`/api/images/${imageId}/plate-solve`, {
  method: 'POST'
});
const plateSolveResult = await result.json();
```

## Environment Variables

Some API behavior can be configured via environment variables:

- `ASTROMETRY_API_KEY` - Default Astrometry.net API key
- `IMMICH_URL` - Default Immich server URL
- `IMMICH_API_KEY` - Default Immich API key
- `ENABLE_PLATE_SOLVING` - Enable/disable plate solving worker

Note: Admin settings take precedence over environment variables.

## Best Practices

1. **Use filtering** - Always filter API responses when possible to reduce payload size
2. **Handle errors** - Implement proper error handling for all API calls
3. **Check health** - Monitor the `/api/health` endpoint for system status
4. **Batch operations** - Use bulk endpoints when processing multiple items
5. **Real-time updates** - Use WebSocket connections for live updates
6. **Asset caching** - Cache image assets appropriately to reduce load

## Support

For API support and questions:
- Check the [GitHub Issues](https://github.com/mstelz/Sidereal/issues)
- Review the [Contributing Guide](../CONTRIBUTING.md)
- Join discussions in [GitHub Discussions](https://github.com/mstelz/Sidereal/discussions)
