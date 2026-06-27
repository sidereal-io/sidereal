DROP INDEX `astrophotography_images_immich_id_unique`;--> statement-breakpoint
ALTER TABLE `astrophotography_images` DROP COLUMN `immich_id`;--> statement-breakpoint
PRAGMA foreign_keys=OFF;--> statement-breakpoint
CREATE TABLE `__new_equipment_group_members` (
	`id` integer PRIMARY KEY NOT NULL,
	`group_id` integer NOT NULL,
	`equipment_id` integer NOT NULL,
	`created_at` integer NOT NULL,
	FOREIGN KEY (`group_id`) REFERENCES `equipment_groups`(`id`) ON UPDATE no action ON DELETE cascade,
	FOREIGN KEY (`equipment_id`) REFERENCES `equipment`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
INSERT INTO `__new_equipment_group_members`("id", "group_id", "equipment_id", "created_at") SELECT "id", "group_id", "equipment_id", "created_at" FROM `equipment_group_members`;--> statement-breakpoint
DROP TABLE `equipment_group_members`;--> statement-breakpoint
ALTER TABLE `__new_equipment_group_members` RENAME TO `equipment_group_members`;--> statement-breakpoint
PRAGMA foreign_keys=ON;--> statement-breakpoint
CREATE TABLE `__new_image_acquisition` (
	`id` integer PRIMARY KEY NOT NULL,
	`image_id` integer NOT NULL,
	`filter_id` integer,
	`filter_name` text,
	`frame_count` integer NOT NULL,
	`exposure_time` real NOT NULL,
	`gain` integer,
	`offset` integer,
	`binning` text,
	`sensor_temp` real,
	`date` integer,
	`notes` text,
	`created_at` integer NOT NULL,
	FOREIGN KEY (`image_id`) REFERENCES `astrophotography_images`(`id`) ON UPDATE no action ON DELETE cascade,
	FOREIGN KEY (`filter_id`) REFERENCES `equipment`(`id`) ON UPDATE no action ON DELETE set null
);
--> statement-breakpoint
INSERT INTO `__new_image_acquisition`("id", "image_id", "filter_id", "filter_name", "frame_count", "exposure_time", "gain", "offset", "binning", "sensor_temp", "date", "notes", "created_at") SELECT "id", "image_id", "filter_id", "filter_name", "frame_count", "exposure_time", "gain", "offset", "binning", "sensor_temp", "date", "notes", "created_at" FROM `image_acquisition`;--> statement-breakpoint
DROP TABLE `image_acquisition`;--> statement-breakpoint
ALTER TABLE `__new_image_acquisition` RENAME TO `image_acquisition`;--> statement-breakpoint
CREATE TABLE `__new_image_equipment` (
	`id` integer PRIMARY KEY NOT NULL,
	`image_id` integer,
	`equipment_id` integer,
	`settings` text,
	`notes` text,
	`created_at` integer NOT NULL,
	FOREIGN KEY (`image_id`) REFERENCES `astrophotography_images`(`id`) ON UPDATE no action ON DELETE cascade,
	FOREIGN KEY (`equipment_id`) REFERENCES `equipment`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
INSERT INTO `__new_image_equipment`("id", "image_id", "equipment_id", "settings", "notes", "created_at") SELECT "id", "image_id", "equipment_id", "settings", "notes", "created_at" FROM `image_equipment`;--> statement-breakpoint
DROP TABLE `image_equipment`;--> statement-breakpoint
ALTER TABLE `__new_image_equipment` RENAME TO `image_equipment`;--> statement-breakpoint
CREATE TABLE `__new_plate_solving_jobs` (
	`id` integer PRIMARY KEY NOT NULL,
	`image_id` integer,
	`astrometry_submission_id` text,
	`astrometry_job_id` text,
	`status` text DEFAULT 'pending' NOT NULL,
	`submitted_at` integer NOT NULL,
	`completed_at` integer,
	`result` text,
	FOREIGN KEY (`image_id`) REFERENCES `astrophotography_images`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
INSERT INTO `__new_plate_solving_jobs`("id", "image_id", "astrometry_submission_id", "astrometry_job_id", "status", "submitted_at", "completed_at", "result") SELECT "id", "image_id", "astrometry_submission_id", "astrometry_job_id", "status", "submitted_at", "completed_at", "result" FROM `plate_solving_jobs`;--> statement-breakpoint
DROP TABLE `plate_solving_jobs`;--> statement-breakpoint
ALTER TABLE `__new_plate_solving_jobs` RENAME TO `plate_solving_jobs`;