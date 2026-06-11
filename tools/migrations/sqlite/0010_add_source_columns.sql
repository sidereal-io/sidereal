ALTER TABLE `astrophotography_images` ADD COLUMN `source_type` text DEFAULT 'immich' NOT NULL;--> statement-breakpoint
ALTER TABLE `astrophotography_images` ADD COLUMN `source_id` text;--> statement-breakpoint
UPDATE `astrophotography_images` SET `source_id` = `immich_id` WHERE `source_id` IS NULL AND `immich_id` IS NOT NULL;
