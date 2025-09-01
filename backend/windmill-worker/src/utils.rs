use windmill_queue::MiniPulledJob;

pub fn get_root_job_id(job: &MiniPulledJob) -> uuid::Uuid {
    // fallback to flow_innermost_root_job and parent_job as root_job is not set if equal to innermost root job or parent job
    job.root_job
        .or(job.flow_innermost_root_job)
        .or(job.parent_job)
        .unwrap_or(job.id)
}
