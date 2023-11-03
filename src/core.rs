//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

pub mod builtins;

use nix::sys::wait;
use nix::sys::wait::WaitStatus;
use nix::{fcntl, unistd};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::process;
use std::os::linux::fs::MetadataExt;
use std::path::Path;
use std::os::fd::RawFd;
use nix::sys::signal;
use nix::sys::signal::{Signal, SigHandler};

pub struct ShellCore {
    pub history: Vec<String>,
    pub flags: String,
    pub vars: HashMap<String, String>, 
    pub builtins: HashMap<String, fn(&mut ShellCore, &mut Vec<String>) -> i32>,
    pub nest: Vec<String>,
    pub input_interrupt: bool,
    pub is_subshell: bool,
    pub tty_fd: RawFd, 
}

fn is_interactive(pid: u32) -> bool {
    let std_path = format!("/proc/{}/fd/0", pid);
    match Path::new(&std_path).metadata() {
        Ok(metadata) => metadata.st_mode() == 8592,
        Err(err) => panic!("{}", err),
    }
}

fn get_tty_fd() -> Option<RawFd> {
    for fd in 0..3 {
        match unistd::isatty(fd) {
            Ok(true) => return Some(fd),
            _ => {},
        }
    }
    None
}

fn ignore_signal(sig: Signal) {
    unsafe { signal::signal(sig, SigHandler::SigIgn) }
        .expect("sush(fatal): cannot ignore signal");
}

fn restore_signal(sig: Signal) {
    unsafe { signal::signal(sig, SigHandler::SigDfl) }
        .expect("sush(fatal): cannot restore signal");
}

fn set_foreground() {
    if let Some(fd) = get_tty_fd(){
        ignore_signal(Signal::SIGTTOU);
        match unistd::tcsetpgrp(fd, unistd::getpid()) {
            Ok(_)  => {},
            Err(_) => panic!("sush(fatal): cannot get the terminal"),
        }
        restore_signal(Signal::SIGTTOU);
    }
}

impl ShellCore {
    pub fn new() -> ShellCore {
        let mut core = ShellCore{
            history: Vec::new(),
            flags: String::new(),
            vars: HashMap::new(),
            builtins: HashMap::new(),
            nest: vec![],
            input_interrupt: false,
            is_subshell: false,
            tty_fd: -1,
        };

        let pid = process::id();
        if is_interactive(pid) {
            core.flags += "i";
        }
        core.vars.insert("$".to_string(), pid.to_string());
        core.vars.insert("BASHPID".to_string(), core.vars["$"].clone());
        core.vars.insert("BASH_SUBSHELL".to_string(), "0".to_string());

        core.vars.insert("?".to_string(), "0".to_string());

        core.builtins.insert("cd".to_string(), builtins::cd);
        core.builtins.insert("exit".to_string(), builtins::exit);

        core.tty_fd = fcntl::fcntl(2, fcntl::F_DUPFD_CLOEXEC(255))
            .expect("Can't allocate fd for tty FD");

        core
    }

    pub fn has_flag(&self, flag: char) -> bool {
        if let Some(_) = self.flags.find(flag) {
            return true;
        }
        false
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

        self.vars.insert("?".to_string(), exit_status.to_string()); //追加
    } 

    pub fn wait_pipeline(&mut self, pids: Vec<Option<Pid>>) {
        if pids.len() == 1 && pids[0] == None {
            return;
        }

        for pid in pids {
            self.wait_process(pid.expect("SUSHI INTERNAL ERROR (no pid)"));
        }

        set_foreground();
    }

    pub fn run_builtin(&mut self, args: &mut Vec<String>) -> bool {
        if ! self.builtins.contains_key(&args[0]) {
            return false;
        }

        let func = self.builtins[&args[0]];
        let status = func(self, args);
        self.vars.insert("?".to_string(), status.to_string());
        true
    }

    pub fn exit(&self) -> ! {
        let exit_status = match self.vars["?"].parse::<i32>() {
            Ok(n)  => n%256, 
            Err(_) => {
                eprintln!("sush: exit: {}: numeric argument required", self.vars["?"]);
                2
            },
        };
    
        process::exit(exit_status)
    }

    pub fn set_subshell_vars(&mut self) {
        let pid = nix::unistd::getpid();
        self.vars.insert("BASHPID".to_string(), pid.to_string());
        match self.vars["BASH_SUBSHELL"].parse::<usize>() {
            Ok(num) => self.vars.insert("BASH_SUBSHELL".to_string(), (num+1).to_string()),
            Err(_) =>  self.vars.insert("BASH_SUBSHELL".to_string(), "0".to_string()),
        };
    }

    pub fn set_pgid(&self, pid: Pid, pgid: Pid) {
        unistd::setpgid(pid, pgid).expect("sush(fatal): cannot set pgid");
        if pid.as_raw() == 0 && pgid.as_raw() == 0 {
            set_foreground();
        }
    }
}
