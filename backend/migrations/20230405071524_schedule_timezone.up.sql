ALTER TABLE schedule ADD COLUMN timezone VARCHAR(255) NOT NULL DEFAULT 'UTC';

-- INSERT the correct IANA timezone string for each offset value

UPDATE schedule SET timezone = 'Pacific/Honolulu' WHERE offset_ = 600;
UPDATE schedule SET timezone = 'America/Anchorage' WHERE offset_ = 540;
UPDATE schedule SET timezone = 'America/Los_Angeles' WHERE offset_ = 480;
UPDATE schedule SET timezone = 'America/Chicago' WHERE offset_ = 360;
UPDATE schedule SET timezone = 'America/New_York' WHERE offset_ = 300;
UPDATE schedule SET timezone = 'America/Halifax' WHERE offset_ = 240;
UPDATE schedule SET timezone = 'America/Sao_Paulo' WHERE offset_ = 180;
UPDATE schedule SET timezone = 'Atlantic/South_Georgia' WHERE offset_ = 120;
UPDATE schedule SET timezone = 'Atlantic/Cape_Verde' WHERE offset_ = 60;
UPDATE schedule SET timezone = 'Europe/London' WHERE offset_ = 0;
UPDATE schedule SET timezone = 'Europe/Berlin' WHERE offset_ = -60;
UPDATE schedule SET timezone = 'Europe/Athens' WHERE offset_ = -120;
UPDATE schedule SET timezone = 'Europe/Moscow' WHERE offset_ = -180;
UPDATE schedule SET timezone = 'Asia/Dubai' WHERE offset_ = -240;
UPDATE schedule SET timezone = 'Asia/Aqtau' WHERE offset_ = -300;
UPDATE schedule SET timezone = 'Asia/Almaty' WHERE offset_ = -360;
UPDATE schedule SET timezone = 'Asia/Bangkok' WHERE offset_ = -420;
UPDATE schedule SET timezone = 'Asia/Hong_Kong' WHERE offset_ = -480;
UPDATE schedule SET timezone = 'Asia/Tokyo' WHERE offset_ = -540;
UPDATE schedule SET timezone = 'Australia/Sydney' WHERE offset_ = -600;

ALTER TABLE schedule DROP COLUMN offset_;