-- Create "accounts" table
CREATE TABLE `accounts` (
  `id` text NOT NULL,
  `status` text NOT NULL,
  `name` text NULL,
  `primary_email` text NOT NULL,
  `created_at` integer NOT NULL,
  `updated_at` integer NOT NULL,
  `version` integer NOT NULL,
  PRIMARY KEY (`id`),
  CHECK (status IN ('Pending', 'Active', 'Suspended'))
);
-- Create "identities" table
CREATE TABLE `identities` (
  `provider` text NOT NULL,
  `subject` text NOT NULL,
  `account_id` text NOT NULL,
  `email` text NOT NULL,
  `email_verified` integer NOT NULL DEFAULT 0,
  `linked_at` integer NOT NULL,
  PRIMARY KEY (`provider`, `subject`)
);
-- Create index "idx_identities_account_id" to table: "identities"
CREATE INDEX `idx_identities_account_id` ON `identities` (`account_id`);
-- Create "processed_events" table
CREATE TABLE `processed_events` (
  `event_id` text NOT NULL,
  `processed_at` integer NOT NULL,
  PRIMARY KEY (`event_id`)
);
-- Create "_atlas_revisions" table
CREATE TABLE `_atlas_revisions` (
  `version` text NOT NULL,
  `applied_at` integer NOT NULL,
  PRIMARY KEY (`version`)
);
