-- Add up migration script here

CREATE TABLE public.outstanding_wait_time (
    job_id uuid NOT NULL,
    self_wait_time_ms bigint,
    aggregate_wait_time_ms bigint
);


-- ALTER TABLE public.outstanding_wait_time OWNER TO postgres;

--
-- Name: outstanding_wait_time outstanding_wait_time_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.outstanding_wait_time
    ADD CONSTRAINT outstanding_wait_time_pkey PRIMARY KEY (job_id);


--
-- Name: TABLE outstanding_wait_time; Type: ACL; Schema: public; Owner: postgres
--

-- GRANT ALL ON TABLE public.outstanding_wait_time TO windmill_user;
-- GRANT ALL ON TABLE public.outstanding_wait_time TO windmill_admin;


--
-- PostgreSQL database dump complete
--

