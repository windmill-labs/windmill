-- Add up migration script here
CREATE TABLE public.v2_job_runtime (
    id uuid NOT NULL,
    ping timestamp with time zone DEFAULT now(),
    memory_peak integer
);

ALTER TABLE public.v2_job_runtime OWNER TO postgres;

ALTER TABLE ONLY public.v2_job_runtime
    ADD CONSTRAINT v2_job_runtime_pkey PRIMARY KEY (id);

ALTER TABLE ONLY public.v2_job_runtime
    ADD CONSTRAINT v2_job_runtime_id_fkey FOREIGN KEY (id) REFERENCES public.v2_job_queue(id) ON DELETE CASCADE;








