//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

pub mod builtins;
pub mod data;
pub mod history;
pub mod jobtable;

use self::data::Data;
use std::collections::HashMap;
use std::os::fd::{FromRawFd, OwnedFd};
use std::{io, env, path, process};
use nix::{fcntl, unistd};
use nix::sys::{signal, wait};
use nix::sys::signal::{Signal, SigHandler};
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use crate::core::jobtable::JobEntry;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

#[derive(Default)]
pub struct ShellCore {
    pub data: Data,
    rewritten_history: HashMap<usize, String>,
    pub history: Vec<String>,
    pub builtins: HashMap<String, fn(&mut ShellCore, &mut Vec<String>) -> i32>,
    pub sigint: Arc<AtomicBool>,
    pub is_subshell: bool,
    pub tty_fd: Option<OwnedFd>,
    pub job_table: Vec<JobEntry>,
    tcwd: Option<path::PathBuf>, // the_current_working_directory
}

fn is_interactive() -> bool {
    match unistd::isatty(0) {
        Ok(result) => result,
        Err(err) => panic!("{}", err),
    }
}

impl ShellCore {
    pub fn new() -> ShellCore {
        let mut core = ShellCore::default();

        core.init_current_directory();
        core.set_initial_parameters();
        core.set_builtins();

        if is_interactive() {
            core.data.flags += "i";
            let fd = fcntl::fcntl(2, fcntl::F_DUPFD_CLOEXEC(255))
                .expect("sush(fatal): Can't allocate fd for tty FD");
            core.tty_fd = Some(unsafe{OwnedFd::from_raw_fd(fd)});
        }

        let home = core.data.get_param("HOME").to_string();
        core.data.set_param("HISTFILE", &(home + "/.sush_history"));
        core.data.set_param("HISTFILESIZE", "2000");

        core
    }

    fn set_initial_parameters(&mut self) {
        self.data.set_param("$", &process::id().to_string());
        self.data.set_param("BASHPID", &process::id().to_string());
        self.data.set_param("BASH_SUBSHELL", "0");
        self.data.set_param("?", "0");
        self.data.set_param("PS1", "\\[\\033[01;36m\\]\\b\\[\\033[00m\\]\\[\\033[01;35m\\]\\w\\[\\033[00m\\](debug)🍣 ");
        self.data.set_param("PS2", "> ");
        self.data.set_param("HOME", &env::var("HOME").unwrap_or("/".to_string()));
    }

    pub fn has_flag(&self, flag: char) -> bool {
        self.data.flags.find(flag) != None 
    }

    pub fn wait_process(&mut self, child: Pid) {
        let exit_status = match wait::waitpid(child, None) {
            Ok(WaitStatus::Exited(_pid, status)) => {
                status
            },
            Ok(WaitStatus::Signaled(pid, signal, _coredump)) => {
                eprintln!("Pid: {:?}, Signal: {:?}", pid, signal);
                128+signal as i32
            },
            Ok(unsupported) => {
                eprintln!("Unsupported: {:?}", unsupported);
                1
            },
            Err(err) => {
                panic!("Error: {:?}", err);
            },
        };

        if exit_status == 130 {
            self.sigint.store(true, Relaxed);
        }
        self.data.parameters.insert("?".to_string(), exit_status.to_string()); //追加
    } 

    fn set_foreground(&self) {
        let fd = match self.tty_fd.as_ref() {
            Some(fd) => fd,
            _        => return,
        };
        let pgid = unistd::getpgid(Some(Pid::from_raw(0)))
                   .expect("sush(fatal): cannot get pgid");

        if unistd::tcgetpgrp(fd) == Ok(pgid) {
            return;
        }

        ignore_signal(Signal::SIGTTOU); //SIGTTOUを無視
        unistd::tcsetpgrp(fd, pgid)
            .expect("sush(fatal): cannot get the terminal");
        restore_signal(Signal::SIGTTOU); //SIGTTOUを受け付け
    }

    pub fn wait_pipeline(&mut self, pids: Vec<Option<Pid>>) {
        if pids.len() == 1 && pids[0] == None {
            return;
        }

        for pid in pids {
            self.wait_process(pid.expect("SUSHI INTERNAL ERROR (no pid)"));
        }
        self.set_foreground();
    }

    pub fn run_builtin(&mut self, args: &mut Vec<String>) -> bool {
        if args.len() == 0 {
            panic!("SUSH INTERNAL ERROR (no arg for builtins)");
        }

        if ! self.builtins.contains_key(&args[0]) {
            return false;
        }

        let func = self.builtins[&args[0]];
        let status = func(self, args);
        self.data.parameters.insert("?".to_string(), status.to_string());
        true
    }

    fn set_subshell_parameters(&mut self) {
        let pid = nix::unistd::getpid();
        self.data.parameters.insert("BASHPID".to_string(), pid.to_string());
        match self.data.parameters["BASH_SUBSHELL"].parse::<usize>() {
            Ok(num) => self.data.parameters.insert("BASH_SUBSHELL".to_string(), (num+1).to_string()),
            Err(_) =>  self.data.parameters.insert("BASH_SUBSHELL".to_string(), "0".to_string()),
        };
    }

    pub fn set_pgid(&self, pid: Pid, pgid: Pid) {
        unistd::setpgid(pid, pgid).expect("sush(fatal): cannot set pgid");
        if pid.as_raw() == 0 && pgid.as_raw() == 0 { //以下3行追加
            self.set_foreground();
        }
    }

    pub fn initialize_as_subshell(&mut self, pid: Pid, pgid: Pid){
        restore_signal(Signal::SIGINT);

        self.is_subshell = true;
        self.set_pgid(pid, pgid);
        self.set_subshell_parameters();
        self.job_table.clear();
    }

    pub fn init_current_directory(&mut self) {
        match env::current_dir() {
            Ok(path) => self.tcwd = Some(path),
            Err(err) => eprintln!("pwd: error retrieving current directory: {:?}", err),
        }
    }

    pub fn get_current_directory(&mut self) -> Option<path::PathBuf> {
        if self.tcwd.is_none() {
            self.init_current_directory();
        }
        self.tcwd.clone()
    }


    pub fn set_current_directory(&mut self, path: &path::PathBuf) -> Result<(), io::Error> {
        let res = env::set_current_dir(path);
        if res.is_ok() {
            self.tcwd = Some(path.clone());
        }
        res
    }
}
