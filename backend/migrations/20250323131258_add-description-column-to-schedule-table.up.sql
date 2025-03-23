-- Add up migration script here
ALTER TABLE schedule
	ADD COLUMN description TEXT NOT NULL;