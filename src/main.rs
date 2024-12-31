//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

mod child;
mod core;
mod feeder;
mod elements;
mod signal;
mod utils;

use builtins::{option, parameter};
use std::{env, process};
use std::sync::atomic::Ordering::Relaxed;
use crate::core::{builtins, ShellCore};
use crate::elements::script::Script;
use crate::feeder::{Feeder, InputError};
use utils::{exit, file_check, arg};

fn show_version() {
    const V: &'static str = env!("CARGO_PKG_VERSION");
    const P: &'static str = env!("CARGO_BUILD_PROFILE");
    eprintln!("Rusty Bash (a.k.a. Sushi shell), version {} - {}
© 2024 Ryuichi Ueda
License: BSD 3-Clause

This is open source software. You can redistirbute and use in source
and binary forms with or without modification under the license.
There is no warranty, to the extent permitted by law.", V, P);
    process::exit(0);
}

fn read_rc_file(core: &mut ShellCore) {
    if ! core.db.flags.contains("i") {
        return;
    }

    let dir = core.db.get_param("CARGO_MANIFEST_DIR").unwrap_or(String::new());// {
        /*
        "" => core.db.get_param("HOME"),
        s  => s.to_string(),
        */
    //};

    let rc_file = dir + "/.sushrc";

    if file_check::is_regular_file(&rc_file) {
        core.run_builtin(&mut vec![".".to_string(), rc_file], &mut vec![]);
    }
}

fn configure(args: &Vec<String>) -> ShellCore {
    let mut core = ShellCore::new();
    let mut parameters = vec![args[0].clone()];
    let mut options = vec![];

    for i in 1..args.len() {
        if args[i].starts_with("-") {
            options.push(args[i].clone());
        }else{
            core.script_name = args[i].clone();
            parameters = args[i..].to_vec();
            break;
        }
    }

    option::set_options(&mut core, &options);
    parameter::set_positions(&mut core, &parameters);
    core
}

fn main() {
    let mut args = arg::dissolve_options(&env::args().collect());
    if args.len() > 1 && args[1] == "--version" {
        show_version();
    }

    let c_parts = arg::consume_with_subsequents("-c", &mut args);
    if c_parts.len() != 0 {
        run_and_exit_c_option(&args, &c_parts);
    }

    let mut core = configure(&args);
    signal::run_signal_check(&mut core);

    if core.script_name == "-" {
        read_rc_file(&mut core);
    }
    main_loop(&mut core);
}

fn set_history(core: &mut ShellCore, s: &str) {
    if core.read_stdin || core.history.len() == 0 {
        return;
    }

    core.history[0] = s.trim_end().replace("\n", "↵ \0").to_string();
    if core.history[0].len() == 0
    || (core.history.len() > 1 && core.history[0] == core.history[1]) {
        core.history.remove(0);
    }
}

fn show_message() {
    const V: &'static str = env!("CARGO_PKG_VERSION");
    const P: &'static str = env!("CARGO_BUILD_PROFILE");
    eprintln!("Rusty Bash (a.k.a. Sushi shell), version {} - {}", V, P);
}

fn main_loop(core: &mut ShellCore) {
    let mut feeder = Feeder::new("");

    if core.script_name != "-" {
        core.db.flags.retain(|f| f != 'i');
        feeder.set_file(&core.script_name);
    }

    if core.db.flags.contains('i') {
        show_message();
    }

    loop {
        core.jobtable_check_status();
        core.jobtable_print_status_change();

        match feeder.feed_line(core) {
            Ok(()) => {}, 
            Err(InputError::Interrupt) => {
                signal::input_interrupt_check(&mut feeder, core);
                continue;
            },
            _ => break,
        }

        core.word_eval_error = false;
        core.sigint.store(false, Relaxed);
        match Script::parse(&mut feeder, core, false){
            Some(mut s) => {
                s.exec(core);
                set_history(core, &s.get_text());
            },
            None => {},
        }
        core.sigint.store(false, Relaxed);
    }
    core.write_history_to_file();
    exit::normal(core);
}

fn run_and_exit_c_option(args: &Vec<String>, c_parts: &Vec<String>) {
    if c_parts.len() < 2 {
        println!("{}: -c: option requires an argument", &args[0]);
        process::exit(2);                
    }

    let mut core = ShellCore::new();
    let mut parameters = if c_parts.len() > 2 {
        c_parts[2..].to_vec()
    }else{
        vec![args[0].clone()]
    };

    option::set_options(&mut core, &mut args[1..].to_vec());
    parameter::set_positions(&mut core, &mut parameters);
    signal::run_signal_check(&mut core);
    core.db.flags.retain(|f| f != 'i');

    core.db.flags += "c";
    if core.db.flags.contains('v') {
        eprintln!("{}", &c_parts[1]);
    }

    let mut feeder = Feeder::new(&c_parts[1]);
    if let Some(mut s) = Script::parse(&mut feeder, &mut core, false){
        s.exec(&mut core);
    }
    exit::normal(&mut core)
}
