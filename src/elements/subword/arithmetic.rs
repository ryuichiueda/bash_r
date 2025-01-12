//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{ShellCore, Feeder};
use crate::elements::command::arithmetic::ArithmeticCommand;
use crate::elements::subword::Subword;

#[derive(Debug, Clone)]
pub struct Arithmetic {
    pub text: String,
    com: ArithmeticCommand,
}

impl Subword for Arithmetic {
    fn get_text(&self) -> &str { &self.text.as_ref() }
    fn boxed_clone(&self) -> Box<dyn Subword> {Box::new(self.clone())}

    fn substitute(&mut self, core: &mut ShellCore) -> Result<(), String> {
        if let Some(s) = self.com.eval(core) {
            self.text = s;
            return Ok(());
        }
        Err("arithmetic operation failed".to_string())
    }
}

impl Arithmetic {
    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Option<Self> {
        if ! feeder.starts_with("$((") {
            return None;
        }
        feeder.set_backup();
        let dl = feeder.consume(1);

        if let Some(a) = ArithmeticCommand::parse(feeder, core) {
            feeder.pop_backup();
            return Some(Arithmetic{ text: dl + &a.text.clone(), com: a});
        }
        feeder.rewind();
        None
    }
}
