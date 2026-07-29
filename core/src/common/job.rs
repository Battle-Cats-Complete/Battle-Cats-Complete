#[derive(Debug, Clone)]
pub enum JobEvent {
    Log(String),
    Progress { current: usize, total: usize },
    Finished(JobOutcome),
}

#[derive(Debug, Clone)]
pub enum JobOutcome {
    Completed,
    Aborted,
    Failed(String),
}
