//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

mod calculator;

use crate::{ShellCore, Feeder};
use self::calculator::calculate;
use super::word::Word;

#[derive(Debug, Clone)]
enum CalcElement {
    UnaryOp(String),
    BinaryOp(String),
    Num(i64),
    Name(String, i32),
    Word(Word, i32),
    LeftParen,
    RightParen,
    PlusPlus,
    MinusMinus,
}

#[derive(Debug, Clone)]
pub struct Calc {
    pub text: String,
    elements: Vec<CalcElement>,
    paren_stack: Vec<char>,
}

impl Calc {
    pub fn eval(&mut self, core: &mut ShellCore) -> Option<String> {
        let es = match self.evaluate_elems(core) {
            Ok(data)     => data, 
            Err(err_msg) => {
                eprintln!("sush: {}", err_msg);
                return None;
            },
        };

        match calculate(&es) {
            Ok(ans)  => Some(ans),
            Err(msg) => {
                eprintln!("sush: {}: {}", &self.text, msg);
                None
            },
        }
    }

    fn evaluate_elems(&self, core: &mut ShellCore) -> Result<Vec<CalcElement>, String> {
        let mut ans = vec![];
        let mut next_inc: i32 = 0;

        for e in self.elements.iter() {
            match e {
                CalcElement::Name(s, inc) => {
                    let val = core.data.get_param(s);
                    match self.value_to_num(&val, "") {
                        Ok(n)        => ans.push(CalcElement::Num(n+next_inc as i64)),
                        Err(err_msg) => return Err(err_msg), 
                    }

                    core.data.set_param(&s, &(val.parse::<i32>().unwrap_or(0) + next_inc + inc).to_string());
                },
                CalcElement::Word(w, inc) => {
                    let val = match w.eval_as_value(core) {
                        Some(v) => v, 
                        None => return Err(format!("{}: wrong substitution", &self.text)),
                    };

                    let mut f = Feeder::new(&val);
                    if f.scanner_name(core) == val.len() {
                        let num = core.data.get_param(&val);
                        let num = match self.value_to_num(&num, &w.text) {
                            Ok(n)        => {ans.push(CalcElement::Num(n+next_inc as i64)); n},
                            Err(err_msg) => return Err(err_msg), 
                        };
                        core.data.set_param(&val, &(num + (next_inc + *inc) as i64).to_string());
                    }else{
                        match self.value_to_num(&val, &w.text) {
                            Ok(n)        => ans.push(CalcElement::Num(n)),
                            Err(err_msg) => return Err(err_msg), 
                        }
                    }
                },
                _ => ans.push(e.clone()),
            }

            next_inc = match e {
                CalcElement::PlusPlus => 1,
                CalcElement::MinusMinus => -1,
                _ => 0, 
            };
        }

        Ok(ans)
    }

    fn value_to_num(&self, val: &String, text: &str) -> Result<i64, String> {
        if text.find('\'').is_some() {
            Err(format!("{0}: syntax error: operand expected (error token is \"{0}\")", &val))
        }else if let Ok(n) = val.parse::<i64>() {
            Ok( n )
        }else {
            Ok( 0 )
        }
    }

    pub fn new() -> Calc {
        Calc {
            text: String::new(),
            elements: vec![],
            paren_stack: vec![],
        }
    }

    fn eat_blank(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) {
        let len = feeder.scanner_multiline_blank(core);
        ans.text += &feeder.consume(len);
    }

    fn eat_integer(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        match &ans.elements.last() {
            Some(CalcElement::Num(_)) => return false,
            _ => {},
        }

        let len = feeder.scanner_nonnegative_integer(core);
        if len == 0 {
            return false;
        }

        let n = match feeder.refer(len).parse::<i64>() {
            Ok(n)  => n, 
            Err(_) => return false,
        };

        ans.inc_dec_to_unarys();
        let s = feeder.consume(len);
        ans.text += &s.clone();
        ans.elements.push( CalcElement::Num(n) );
        true
    }

    fn eat_name(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        let len = feeder.scanner_name(core);
        if len == 0 {
            return false;
        }

        let s = feeder.consume(len);
        ans.text += &s;
        Self::eat_blank(feeder, ans, core);

        if feeder.starts_with("++") {
            ans.elements.push( CalcElement::Name(s.clone(), 1) );
            ans.text += &feeder.consume(2);
        } else if feeder.starts_with("--") {
            ans.elements.push( CalcElement::Name(s.clone(), -1) );
            ans.text += &feeder.consume(2);
        } else{
            ans.elements.push( CalcElement::Name(s.clone(), 0) );
        }

        true
    }

    fn eat_incdec(feeder: &mut Feeder, ans: &mut Self) -> bool {
        if feeder.starts_with("++") {
            ans.text += &feeder.consume(2);
            ans.elements.push( CalcElement::PlusPlus );
        }else if feeder.starts_with("--") {
            ans.text += &feeder.consume(2);
            ans.elements.push( CalcElement::MinusMinus );
        }else {
            return false;
        };
        true
    }

    fn inc_dec_to_unarys(&mut self) {
        let pm = match self.elements.last() {
            Some(CalcElement::PlusPlus) => "+",
            Some(CalcElement::MinusMinus) => "-",
            _ => return,
        }.to_string();

        self.elements.pop();

        match self.elements.last() {
            None |
            Some(CalcElement::UnaryOp(_)) |
            Some(CalcElement::BinaryOp(_)) |
            Some(CalcElement::LeftParen) 
               => self.elements.push(CalcElement::UnaryOp(pm.clone())),
            _  => self.elements.push(CalcElement::BinaryOp(pm.clone())),
        }
        self.elements.push(CalcElement::UnaryOp(pm));
    }

    fn eat_word(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        let mut word = match Word::parse(feeder, core) {
            Some(w) => {
                ans.text += &w.text;
                w
            },
            _ => return false,
        };

        let size = word.subwords.len();
        if size > 2 {
            if (word.subwords[size-1].get_text() == "+" && word.subwords[size-2].get_text() == "+" )
            || (word.subwords[size-1].get_text() == "-" && word.subwords[size-2].get_text() == "-" ) {
                word.subwords.pop();
                word.subwords.pop();
                word.text.pop();
                word.text.pop();
                ans.elements.push( CalcElement::Word(word, 1) );
                return true;
            }
        }

        Self::eat_blank(feeder, ans, core);

        if feeder.starts_with("++") {
            ans.elements.push( CalcElement::Word(word, 1) );
            ans.text += &feeder.consume(2);
        } else if feeder.starts_with("--") {
            ans.elements.push( CalcElement::Word(word, -1) );
            ans.text += &feeder.consume(2);
        } else{
            ans.elements.push( CalcElement::Word(word, 0) );
        }
        true
    }

    fn eat_unary_operator(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        match &ans.elements.last() {
            Some(CalcElement::Num(_)) => return false,
            Some(CalcElement::Name(_, _)) => return false,
            Some(CalcElement::Word(_, _)) => return false,
            _ => {},
        }

        let len = feeder.scanner_calc_operator(core);
        if len == 0 {
            return false;
        }

        let s = if feeder.starts_with("+") || feeder.starts_with("-") {
            feeder.consume(1)
        }else{
            return false
        };

        ans.inc_dec_to_unarys();
        ans.text += &s.clone();
        ans.elements.push( CalcElement::UnaryOp(s) );
        true
    }

    fn eat_paren(feeder: &mut Feeder, ans: &mut Self) -> bool {
        if feeder.starts_with("(") {
            ans.inc_dec_to_unarys();
            ans.paren_stack.push( '(' );
            ans.elements.push( CalcElement::LeftParen );
            ans.text += &feeder.consume(1);
            return true;
        }

        if feeder.starts_with(")") {
            if let Some('(') = ans.paren_stack.last() {
                ans.inc_dec_to_unarys();
                ans.paren_stack.pop();
                ans.elements.push( CalcElement::RightParen );
                ans.text += &feeder.consume(1);
                return true;
            }
        }

        false
    }

    fn eat_binary_operator(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        let len = feeder.scanner_calc_operator(core);
        if len == 0 {
            return false;
        }

        ans.inc_dec_to_unarys();
        let s = feeder.consume(len);
        ans.text += &s.clone();
        ans.elements.push( CalcElement::BinaryOp(s) );
        true
    }

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Option<Calc> {
        let mut ans = Calc::new();

        loop {
            Self::eat_blank(feeder, &mut ans, core);
            if Self::eat_name(feeder, &mut ans, core) 
            || Self::eat_incdec(feeder, &mut ans) 
            || Self::eat_unary_operator(feeder, &mut ans, core)
            || Self::eat_paren(feeder, &mut ans)
            || Self::eat_binary_operator(feeder, &mut ans, core)
            || Self::eat_integer(feeder, &mut ans, core) 
            || Self::eat_word(feeder, &mut ans, core) { 
                continue;
            }

            if feeder.len() != 0 || ! feeder.feed_additional_line(core) {
                break;
            }
        }

        match feeder.starts_with("))") {
            true  => Some(ans),
            false => None,
        }
    }
}
