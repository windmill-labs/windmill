INSERT INTO public.v2_job (
    id, workspace_id, created_by, created_at, kind, script_lang
) VALUES (
    '1eecb96a-c8b0-4a3d-b1b6-087878c55e41', 'test-workspace', 'test-user', '2023-01-01 00:00:00', 'script', 'postgresql'
);

INSERT INTO public.v2_job_completed (
  id, workspace_id, duration_ms, status, result_columns, result
) VALUES (
  '1eecb96a-c8b0-4a3d-b1b6-087878c55e41', 'test-workspace', 1000, 'success'::job_status, '{b,a}', '[{"a": "second", "b": "first"}]'
)