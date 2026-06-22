-- A `data_pipeline` draft bundles every unsaved pipeline script of a folder
-- into a single row keyed at the folder path (typ has no deployed backing
-- table — see UserDraftItemKind::deployed_table). Lets the asset-graph view
-- store its in-flight drafts in the per-user DB draft sync instead of
-- browser-local storage.
ALTER TYPE DRAFT_KIND ADD VALUE IF NOT EXISTS 'data_pipeline';
