-- Add up migration script here
CREATE INDEX IF NOT EXISTS idx_metrics_id_created_at ON public.metrics (id, created_at DESC) WHERE id LIKE 'queue_%';