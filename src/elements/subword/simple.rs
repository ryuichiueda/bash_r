//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{ShellCore, Feeder};
use crate::elements::subword::{Subword, SubwordType};

#[derive(Debug, Clone)]
pub struct SimpleSubword {
    text: String,
    subword_type: SubwordType,
}

impl Subword for SimpleSubword {
    fn get_text(&self) -> &str {&self.text.as_ref()}
    fn boxed_clone(&self) -> Box<dyn Subword> {Box::new(self.clone())}
    fn merge(&mut self, right: &Box<dyn Subword>) { self.text += &right.get_text(); }

    fn set(&mut self, subword_type: SubwordType, s: &str){
        self.text = s.to_string();
        self.subword_type = subword_type;
    }

    fn substitute(&mut self, core: &mut ShellCore) -> bool {
        match self.subword_type {
            SubwordType::Parameter => {
                let value = core.data.get_param(&self.text[1..]);
                self.text = value.to_string();
            },
            _ => {},
        }
        true
    }

    fn make_glob_string(&mut self) -> String { self.text.clone() }
    fn make_unquoted_string(&mut self) -> String { self.text.clone() }
    fn get_type(&self) -> SubwordType { self.subword_type.clone()  }
    fn clear(&mut self) { self.text = String::new(); }
}

impl SimpleSubword {
    pub fn new(s: &str, tp: SubwordType) -> SimpleSubword {
        SimpleSubword {
            text: s.to_string(),
            subword_type: tp,
        }
    }

    pub fn replace_expansion(feeder: &mut Feeder, core: &mut ShellCore) -> bool {
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

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Option<SimpleSubword> {
        if Self::replace_expansion(feeder, core) {
            return Self::parse(feeder, core);
        }

        let len = feeder.scanner_dollar_special_and_positional_param(core);
        if len > 0 {
            return Some(Self::new(&feeder.consume(len), SubwordType::Parameter));
        }

        let len = feeder.scanner_name(core);
        if len > 0 {
            return Some(Self::new(&feeder.consume(len), SubwordType::VarName));
        }

        let len = feeder.scanner_subword_symbol();
        if len > 0 {
            return Some(Self::new(&feeder.consume(len), SubwordType::Symbol));
        }

        let len = feeder.scanner_subword();
        if len > 0 {
            return Some(Self::new(&feeder.consume(len), SubwordType::Other));
        }

        None
    }
}
