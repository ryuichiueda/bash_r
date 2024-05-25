//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

pub mod simple;
mod single_quoted;
mod braced_param;
mod command;
mod escaped_char;
mod ext_glob;
mod double_quoted;
pub mod parameter;
mod varname;

use crate::{ShellCore, Feeder};
use self::simple::SimpleSubword;
use self::braced_param::BracedParam;
use self::command::CommandSubstitution;
use self::escaped_char::EscapedChar;
use self::ext_glob::ExtGlob;
use self::double_quoted::DoubleQuoted;
use self::single_quoted::SingleQuoted;
use self::parameter::Parameter;
use self::varname::VarName;
use std::fmt;
use std::fmt::Debug;

impl Debug for dyn Subword {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct(&self.get_text()).finish()
    }
}

impl Clone for Box::<dyn Subword> {
    fn clone(&self) -> Box<dyn Subword> {
        self.boxed_clone()
    }
}

fn split_str(s: &str) -> Vec<&str> {
    let mut esc = false;
    let mut from = 0;
    let mut pos = 0;
    let mut ans = vec![];

    for c in s.chars() {
        pos += c.len_utf8();
        if esc || c == '\\' {
            esc = ! esc;
            continue;
        }

        if c == ' ' || c == '\t' || c == '\n' {
            ans.push(&s[from..pos-1]);
            from = pos;
        }
    }

    ans.push(&s[from..]);
    ans
}

pub trait Subword {
    fn get_text(&self) -> &str;
    fn set_text(&mut self, _: &str) {}
    fn boxed_clone(&self) -> Box<dyn Subword>;
    fn substitute(&mut self, _: &mut ShellCore) -> bool {true}

    fn split(&self, _core: &mut ShellCore) -> Vec<Box<dyn Subword>>{
        let f = |s| Box::new( SimpleSubword {text: s}) as Box<dyn Subword>;

        split_str(self.get_text()).iter().map(|s| f(s.to_string())).collect()
    }

    fn make_glob_string(&mut self) -> String {self.get_text().to_string()}
    fn make_unquoted_string(&mut self) -> String { self.get_text().to_string() }
    fn is_name(&self) -> bool {false}
    fn is_quoted(&self) -> bool {false}
    fn get_child_subwords(&self) -> Option<&Vec<Box<dyn Subword>>> { None }
}

fn replace_history_expansion(feeder: &mut Feeder, core: &mut ShellCore) -> bool {
    let len = feeder.scanner_history_expansion(core);
    if len == 0 {
        return false;
    }

    let history_len = core.history.len();
    if history_len < 2 {
        feeder.replace(len, "");
        return true;
    }

    let mut his = String::new();
    for h in &core.history[1..] {
        let last = h.split(" ").last().unwrap();

        if ! last.starts_with("!$") {
            his = last.to_string();
            break;
        }
    }

    feeder.replace(len, &his);
    true
}

pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Option<Box<dyn Subword>> {
    if replace_history_expansion(feeder, core) {
        return parse(feeder, core);
    }

    if let Some(a) = BracedParam::parse(feeder, core){ Some(Box::new(a)) }
    else if let Some(a) = CommandSubstitution::parse(feeder, core){ Some(Box::new(a)) }
    else if let Some(a) = SingleQuoted::parse(feeder, core){ Some(Box::new(a)) }
    else if let Some(a) = DoubleQuoted::parse(feeder, core){ Some(Box::new(a)) }
    else if let Some(a) = ExtGlob::parse(feeder, core){ Some(Box::new(a)) }
    else if let Some(a) = EscapedChar::parse(feeder, core){ Some(Box::new(a)) }
    else if let Some(a) = Parameter::parse(feeder, core){ Some(Box::new(a)) }
    else if let Some(a) = VarName::parse(feeder, core){ Some(Box::new(a)) }
    else if let Some(a) = SimpleSubword::parse(feeder){ Some(Box::new(a)) }
    else{ None }
}
