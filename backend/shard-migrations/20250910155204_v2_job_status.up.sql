-- Add up migration script here

CREATE TABLE public.v2_job_status (
    id uuid NOT NULL,
    flow_status jsonb,
    flow_leaf_jobs jsonb,
    workflow_as_code_status jsonb
);


-- ALTER TABLE public.v2_job_status OWNER TO postgres;

--
-- Name: v2_job_status v2_job_status_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job_status
    ADD CONSTRAINT v2_job_status_pkey PRIMARY KEY (id);


--
-- Name: v2_job_status v2_job_status_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.v2_job_status
    ADD CONSTRAINT v2_job_status_id_fkey FOREIGN KEY (id) REFERENCES public.v2_job_queue(id) ON DELETE CASCADE;


--
-- Name: v2_job_status admin_policy; Type: POLICY; Schema: public; Owner: postgres
--

-- CREATE POLICY admin_policy ON public.v2_job_status TO windmill_admin;


--
-- Name: TABLE v2_job_status; Type: ACL; Schema: public; Owner: postgres
--

--GRANT ALL ON TABLE public.v2_job_status TO windmill_user;
-- GRANT ALL ON TABLE public.v2_job_status TO windmill_admin;


--
-- PostgreSQL database dump complete
--


