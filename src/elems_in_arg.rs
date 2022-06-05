//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::debuginfo::DebugInfo;
use crate::CommandPart;
use crate::ShellCore;
use crate::Feeder;
use crate::elems_in_command::Arg;
use crate::elems_executable::{CommandWithArgs, Executable};
use nix::unistd::{pipe}; 

pub trait ArgElem {
    fn eval(&mut self, _conf: &mut ShellCore) -> Vec<String> {
        vec!()
    }

    fn text(&self) -> String;
}

pub struct VarName {
    pub text: String,
    pub pos: DebugInfo,
}

impl VarName {
    pub fn new(text: &mut Feeder, length: usize) -> VarName{
        VarName{
            text: text.consume(length),
            pos: DebugInfo::init(text),
        }
    }
}

impl ArgElem for VarName {
    fn text(&self) -> String {
        self.text.clone()
    }

    fn eval(&mut self, _conf: &mut ShellCore) -> Vec<String> {
        vec!(self.text.clone())
    }
}

pub struct SubArgNonQuoted {
    pub text: String,
    pub pos: DebugInfo,
}

impl ArgElem for SubArgNonQuoted {
    fn text(&self) -> String {
        self.text.clone()
    }

    fn eval(&mut self, _conf: &mut ShellCore) -> Vec<String> {
        vec!(self.text.clone())
    }
}

pub struct SubArgDoubleQuoted {
    pub text: String,
    pub pos: DebugInfo,
    pub subargs: Vec<Box<dyn ArgElem>>
}

impl ArgElem for SubArgDoubleQuoted {
    fn eval(&mut self, conf: &mut ShellCore) -> Vec<String> {
        let mut text = "".to_string();
        for a in &mut self.subargs {
            let sub = a.eval(conf);
            text += &sub[0];
        };

        let s = text.replace("\\", "\\\\").replace("*", "\\*"); 
        vec!(s)
    }

    fn text(&self) -> String {
        self.text.clone()
    }

    /*
    fn get_length(&self) -> usize {
        self.text.len()
    }
    */
}

pub struct SubArgSingleQuoted {
    pub text: String,
    pub pos: DebugInfo,
}

impl ArgElem for SubArgSingleQuoted {
    fn eval(&mut self, _conf: &mut ShellCore) -> Vec<String> {
        let strip = self.text[1..self.text.len()-1].to_string();
        let s = strip.replace("\\", "\\\\").replace("*", "\\*"); 
        vec!(s)
    }

    fn text(&self) -> String {
        self.text.clone()
    }
}

pub struct SubArgBraced {
    pub text: String,
    pub pos: DebugInfo,
    pub args: Vec<Arg>
}

impl ArgElem for SubArgBraced {
    fn eval(&mut self, conf: &mut ShellCore) -> Vec<String> {
        if self.args.len() == 0{
            return vec!("{}".to_string());
        }else if self.args.len() == 1{
            let mut ans = "{".to_string();
            for s in self.args[0].eval(conf) {
                ans += &s;
            };
            ans += "}";
            return vec!(ans);
        };

        let mut ans = vec!();
        for arg in &mut self.args {
            ans.append(&mut arg.eval(conf));
        };
        ans
    }

    fn text(&self) -> String {
        self.text.clone()
    }
}

pub struct SubArgVariable {
    pub text: String,
    pub pos: DebugInfo,
}

impl ArgElem for SubArgVariable {
    fn eval(&mut self, conf: &mut ShellCore) -> Vec<String> {
        let name = if self.text.rfind('}') == Some(self.text.len()-1) {
            self.text[2..self.text.len()-1].to_string()
        }else{
            self.text[1..].to_string()
        };
        vec!(conf.get_var(&name))
    }

    fn text(&self) -> String {
        self.text.clone()
    }
}

pub struct SubArgCommandExp {
    pub text: String,
    pub pos: DebugInfo,
    pub com: CommandWithArgs, 
}

impl ArgElem for SubArgCommandExp {
    fn eval(&mut self, conf: &mut ShellCore) -> Vec<String> {
        self.com.as_string = true;
        let p = pipe().expect("Pipe cannot open");
        self.com.pipe_infd = p.0;
        self.com.pipe_outfd = p.1;
        vec!(self.com.exec(conf).replace("\n", " "))
    }

    fn text(&self) -> String {
        self.text.clone()
    }
}
