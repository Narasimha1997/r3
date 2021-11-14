use crate::mm;

#[derive(Clone, Debug)]
pub struct PID(u64);

#[derive(Debug, Clone)]
pub enum ProcessState {
    Running,
    Terminated,
    Waiting,
}

#[derive(Debug, Clone)]
pub struct ProcessMetadata {
    pub pid: PID,
    pub state: ProcessState,
}
