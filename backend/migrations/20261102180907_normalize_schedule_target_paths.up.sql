-- A schedule's target is compared with the path of the jobs it queues, and with a renamed
-- runnable's old path, neither of which starts with `/`. A target with more than one leading `/`
-- names nothing and is left as is: firing the schedule refuses it.
UPDATE schedule SET script_path = substr(script_path, 2)
WHERE script_path LIKE '/%' AND script_path NOT LIKE '//%';
-- A restore puts the trashed row back as saved, so the copies awaiting one get the same fix.
UPDATE trashbin
SET item_data = jsonb_set(item_data, '{row,script_path}',
    to_jsonb(substr(item_data #>> '{row,script_path}', 2)))
WHERE item_kind = 'schedule'
    AND item_data #>> '{row,script_path}' LIKE '/%'
    AND item_data #>> '{row,script_path}' NOT LIKE '//%';
