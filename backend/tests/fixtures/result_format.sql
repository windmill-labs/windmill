INSERT INTO public.completed_job (
  id, workspace_id, created_by, created_at, duration_ms, success, flow_status, result, job_kind, language
) VALUES (
  '1eecb96a-c8b0-4a3d-b1b6-087878c55e41', 'test-workspace', 'test-user', '2023-01-01 00:00:00', 1000, true, '{"_metadata": {"column_order": ["b", "a"]}}', '[{"a": "second", "b": "first"}]', 'script', 'postgresql'
)