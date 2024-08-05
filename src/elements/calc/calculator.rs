//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::ShellCore;
use super::{Calc, CalcElement};
use super::syntax_error_msg;
use super::word_manip;

fn exponent_error_msg(num: i64) -> String {
    format!("exponent less than 0 (error token is \"{}\")", num)
}

fn assignment_error_msg(right: &str) -> String {
    format!("attempted assignment to non-variable (error token is \"{}\")", right)
}

fn op_order(op: &CalcElement) -> u8 {
    match op {
        CalcElement::PlusPlus | CalcElement::MinusMinus => 14,
        CalcElement::UnaryOp(s) => {
            match s.as_str() {
                "-" | "+" => 13,
                _         => 12,
            }
        },
        CalcElement::BinaryOp(s) => {
            match s.as_str() {
                "**"            => 11, 
                "*" | "/" | "%" => 10, 
                "+" | "-"       => 9, 
                "<<" | ">>"     => 8, 
                "<=" | ">=" | ">" | "<" => 7, 
                "==" | "!="     => 6, 
                "&"             => 5, 
                "^"             => 4, 
                _               => 3,
            }
        },
        CalcElement::ConditionalOp(_, _) => 2,
        _ => 0, 
    }
}

fn to_string(op: &CalcElement) -> String {
    match op {
        CalcElement::Operand(n) => n.to_string(),
        CalcElement::Word(w, inc) => {
            match inc {
                1  => w.text.clone() + "++",
                -1 => w.text.clone() + "--",
                _  => w.text.clone(),
            }
        },
        CalcElement::UnaryOp(s) => s.clone(),
        CalcElement::BinaryOp(s) => s.clone(),
        CalcElement::LeftParen => "(".to_string(),
        CalcElement::RightParen => ")".to_string(),
        CalcElement::Increment(1) => "++".to_string(),
        CalcElement::Increment(-1) => "--".to_string(),
        _ => "".to_string(),
    }
}

fn rev_polish(elements: &Vec<CalcElement>) -> Result<Vec<CalcElement>, CalcElement> {
    let mut ans = vec![];
    let mut stack = vec![];
    let mut last = None;

    for e in elements {
        let ok = match e {
            CalcElement::Operand(_) | CalcElement::Word(_, _) => {ans.push(e.clone()); true},
            CalcElement::LeftParen   => {stack.push(e.clone()); true},
            CalcElement::RightParen  => rev_polish_paren(&mut stack, &mut ans),
            op                       => rev_polish_op(&op, &mut stack, &mut ans),
        };

        if !ok {
            return Err(e.clone());
        }

        match (last, e) {
            ( Some(CalcElement::LeftParen), CalcElement::RightParen ) => return Err(e.clone()),
            _ => {},
        }

        last = Some(e.clone());
    }

    while stack.len() > 0 {
        ans.push(stack.pop().unwrap());
    }

    Ok(ans)
}

fn rev_polish_paren(stack: &mut Vec<CalcElement>, ans: &mut Vec<CalcElement>) -> bool {
    loop {
        match stack.last() {
            None => return false, 
            Some(CalcElement::LeftParen) => {
                stack.pop();
                return true;
            },
            Some(_) => ans.push(stack.pop().unwrap()),
        }
    }
}

fn rev_polish_op(elem: &CalcElement,
                 stack: &mut Vec<CalcElement>, ans: &mut Vec<CalcElement>) -> bool {
    loop {
        match stack.last() {
            None | Some(CalcElement::LeftParen) => {
                stack.push(elem.clone());
                break;
            },
            Some(_) => {
                let last = stack.last().unwrap();
                if op_order(last) <= op_order(elem) {
                    stack.push(elem.clone());
                    break;
                }
                ans.push(stack.pop().unwrap());
            },
        }
    }

    true
}

fn pop_operands(num: usize, stack: &mut Vec<CalcElement>,
                core: &mut ShellCore) -> Result<Vec<i64>, String> {
    let mut ans = vec![];

    for _ in 0..num {
        let n = match stack.pop() {
            Some(CalcElement::Operand(s)) => s,
            Some(CalcElement::Word(w, inc)) => {
                match word_manip::to_operand(&w, 0, inc, core) {
                    Ok(CalcElement::Operand(n)) => n,
                    Err(e)                      => return Err(e),
                    _ => panic!("SUSH INTERNAL ERROR: word_to_operand"),
                }
            },
            _ => return Ok(vec![]),
        };
        ans.push(n);
    }
    Ok(ans)
}

fn bin_operation(op: &str, stack: &mut Vec<CalcElement>, core: &mut ShellCore) -> Result<(), String> {
    match op {
    "=" | "*=" | "/=" | "%=" | "+=" | "-=" | "<<=" | ">>=" | "&=" | "^=" | "|=" 
          => substitution(op, stack, core),
        _ => bin_calc_operation(op, stack, core),
    }


}

fn substitution(op: &str, stack: &mut Vec<CalcElement>, core: &mut ShellCore)-> Result<(), String> {
    let right = match pop_operands(1, stack, core) {
        Ok(v) => {
            match v.len() == 1 {
                true  => v[0],
                false => return Err( syntax_error_msg(op) ),
            }
        },
        Err(e)  => return Err(e),
    };

    let left = match stack.pop() {
        Some(CalcElement::Word(w, 0)) => w,
        Some(CalcElement::Word(_, _)) => return Err( assignment_error_msg(op) ),
        _ => return Err( assignment_error_msg(op) ),
    };

    match word_manip::substitute(op, &left, right, core) {
        Ok(elem) => stack.push(elem),
        Err(msg) => return Err(msg),
    }
    Ok(())
}

fn bin_calc_operation(op: &str, stack: &mut Vec<CalcElement>, core: &mut ShellCore) -> Result<(), String> {
    let (left, right) = match pop_operands(2, stack, core) {
        Ok(v) => {
            match v.len() == 2 {
                true  => (v[1], v[0]), 
                false => return Err( syntax_error_msg(op) ),
            }
        },
        Err(e)  => return Err(e),
    };

    let bool_to_01 = |b| { if b { 1 } else { 0 } };

    let ans = match op {
        "+"  => left + right,
        "-"  => left - right,
        "*"  => left * right,
        "&"  => left & right,
        "^"  => left ^ right,
        "&&"  => bool_to_01( left != 0 && right != 0 ),
        "||"  => bool_to_01( left != 0 || right != 0 ),
        "<<"  => if right < 0 {0} else {left << right},
        ">>"  => if right < 0 {0} else {left >> right},
        "<="  => bool_to_01( left <= right ),
        ">="  => bool_to_01( left >= right ),
        "<"  => bool_to_01( left < right ),
        ">"  => bool_to_01( left > right ),
        "=="  => bool_to_01( left == right ),
        "!="  => bool_to_01( left != right ),
        "%" | "/" => {
            if right == 0 {
                return Err("divided by 0".to_string());
            }
            match op {
                "%" => left % right,
                _   => left / right,
            }
        },
        "**" => {
            if right >= 0 {
                let r = right.try_into().unwrap();
                left.pow(r)
            }else{
                return Err( exponent_error_msg(right) );
            }
        },
        _    => panic!("SUSH INTERNAL ERROR: unknown binary operator"),
    };

    stack.push(CalcElement::Operand(ans));
    Ok(())
}

fn unary_operation(op: &str, stack: &mut Vec<CalcElement>, core: &mut ShellCore) -> Result<(), String> {
    let num = match pop_operands(1, stack, core) {
        Ok(v) => {
            match v.len() == 1 {
                true  => v[0],
                false => return Err( syntax_error_msg(op) ),
            }
        },
        Err(e)  => return Err(e),
    };

    match op {
        "+"  => stack.push( CalcElement::Operand(num) ),
        "-"  => stack.push( CalcElement::Operand(-num) ),
        "!"  => stack.push( CalcElement::Operand(if num == 0 { 1 } else { 0 }) ),
        "~"  => stack.push( CalcElement::Operand( !num ) ),
        _ => panic!("SUSH INTERNAL ERROR: unknown unary operator"),
    }

    Ok(())
}

fn cond_operation(left: &Option<Calc>, right: &Option<Calc>,
    stack: &mut Vec<CalcElement>, core: &mut ShellCore) -> Result<(), String> {
    let num = match pop_operands(1, stack, core) {
        Ok(v) => {
            match v.len() == 1 {
                true  => v[0],
                false => return Err( syntax_error_msg("?") ),
            }
        },
        Err(e)  => return Err(e),
    };

    let mut left = match left {
        Some(c) => c.clone(),
        None    => return Err("expr not found".to_string()),
    };
    let mut right = match right {
        Some(c) => c.clone(),
        None    => return Err("expr not found".to_string()),
    };

    let ans = match num {
        0 => {
            match right.eval_in_cond(core) {
                Ok(num) => num,
                Err(e)  => return Err(e),
            }
        },
        _ => {
            match left.eval_in_cond(core) {
                Ok(num) => num,
                Err(e)  => return Err(e),
            }
        },
    };

    stack.push( CalcElement::Operand( ans ) );
    Ok(())
}

pub fn calculate(elements: &Vec<CalcElement>, core: &mut ShellCore) -> Result<i64, String> {
    if elements.len() == 0 {
        return Ok(0);
    }

    let rev_pol = match rev_polish(&elements) {
        Ok(ans) => ans,
        Err(e)  => return Err( syntax_error_msg(&to_string(&e)) ),
    };

    let mut stack = vec![];

    for e in rev_pol {
        let result = match e {
            CalcElement::Operand(_) | CalcElement::Word(_, _) => {
                stack.push(e.clone());
                Ok(())
            },
            CalcElement::BinaryOp(ref op) => bin_operation(&op, &mut stack, core),
            CalcElement::UnaryOp(ref op)  => unary_operation(&op, &mut stack, core),
            CalcElement::PlusPlus         => inc(1, &mut stack, core),
            CalcElement::MinusMinus       => inc(-1, &mut stack, core),
            CalcElement::ConditionalOp(left, right) => cond_operation(&left, &right, &mut stack, core),
            _ => Err( syntax_error_msg(&to_string(&e)) ),
        };

        if let Err(err_msg) = result {
            return Err(err_msg);
        }
    }

    if stack.len() != 1 {
        return Err( format!("unknown syntax error (stack inconsistency)",) );
    }

    match stack.pop() {
        Some(CalcElement::Operand(n)) => Ok(n),
        Some(CalcElement::Word(w, inc)) => {
            match word_manip::to_operand(&w, 0, inc, core) {
                Ok(CalcElement::Operand(n)) => Ok(n),
                Err(e) => Err(e),
                _      => Err("unknown word parse error".to_string()),
            }
        },
        _ => Err( format!("unknown syntax error",) ),
    }
}

fn inc(inc: i64, stack: &mut Vec<CalcElement>, core: &mut ShellCore) -> Result<(), String> {
    match stack.pop() {
        Some(CalcElement::Word(w, inc_post)) => {
            match word_manip::to_operand(&w, inc, inc_post, core) {
                Ok(op) => {
                    stack.push(op);
                    Ok(())
                },
                Err(e) => Err(e),
            }
        },
        _ => Err("invalid increment".to_string()),
    }
}
