DROP TRIGGER usr_deployers_rename_user ON usr;
DROP FUNCTION usr_deployers_rename();
DROP TRIGGER usr_deployers_forget_folder ON folder;
DROP TRIGGER usr_deployers_forget_group ON group_;
DROP TRIGGER usr_deployers_forget_user ON usr;
DROP FUNCTION usr_deployers_forget();
ALTER TABLE usr DROP COLUMN deployers;
