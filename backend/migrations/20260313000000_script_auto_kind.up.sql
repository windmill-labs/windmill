ALTER TABLE script ADD COLUMN auto_kind VARCHAR(20);
UPDATE script SET auto_kind = 'lib' WHERE no_main_func = true;
ALTER TABLE script DROP COLUMN no_main_func;
