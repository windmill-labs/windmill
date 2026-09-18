-- The files a user message carried, as object-storage references: `[{input, s3, storage?,
-- filename?}]`. Only references, never file bytes and never a presigned URL, so a
-- transcript can show a message's files without reading its run's args.
ALTER TABLE flow_conversation_message ADD COLUMN attachments JSONB;
