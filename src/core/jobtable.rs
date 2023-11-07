//SPDX-FileCopyrightText: 2023 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use nix::unistd::Pid;

#[derive(Debug)]
enum JobStatus {
    Running,
    Finished,
}

#[derive(Debug)]
pub struct JobEntry {
    pids: Vec<(Pid, JobStatus)>,
    status: JobStatus,
    text: String,
}

impl JobEntry {
    pub fn new(pids: Vec<Option<Pid>>, text: &str) -> JobEntry {
        JobEntry {
            pids: pids.into_iter().flatten().map(|e| (e, JobStatus::Running)).collect(),
            status: JobStatus::Running,
            text: text.to_string(),
        }
    }
}
