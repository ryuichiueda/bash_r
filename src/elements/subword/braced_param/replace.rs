//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::ShellCore;
use crate::elements::subword::braced_param::Word;
use crate::utils::glob;

#[derive(Debug, Clone, Default)]
pub struct Replace {
    pub head_only_replace: bool,
    pub tail_only_replace: bool,
    pub all_replace: bool,
    pub replace_from: Option<Word>,
    pub replace_to: Option<Word>,
    pub has_replace_to: bool,
}

impl Replace {
    pub fn set(&self, text: &mut String, core: &mut ShellCore) -> bool {
        let pattern = match &self.replace_from {
            None => return true,
            Some(w) => {
                match w.eval_for_case_word(core) {
                    Some(s) => s,
                    None    => return false,
                }
            },
        };
    
        let string_to = match &self.replace_to {
            None => "".to_string(),
            Some(w) => {
                match w.eval_for_case_word(core) {
                    Some(s) => s,
                    None => match w.subwords.len() {
                        0 => "".to_string(),
                        _ => return false,
                    },
                }
            },
        };
    
        let extglob = core.shopts.query("extglob");
    
        let mut start = 0;
        let mut ans = String::new();
        let mut skip = 0;
        for ch in text.chars() {
            if start != 0 && self.head_only_replace {
                return true;
            }
            if skip > 0 {
                skip -= 1;
                start += ch.len_utf8();
                continue;
            }
    
            let pat = glob::parse(&pattern, extglob);
            let len = glob::longest_match_length(&text[start..].to_string(), &pat);
            if len != 0 && self.tail_only_replace {
                if len == text[start..].len() {
                    *text = [&text[..start], &string_to[0..] ].concat();
                    return true;
                }else{
                    ans += &ch.to_string();
                    start += ch.len_utf8();
                    continue;
                }
            } else if len != 0 && ! self.all_replace {
                *text = [&text[..start], &string_to[0..], &text[start+len..] ].concat();
                return true;
            }
    
            if len != 0 {
                skip = text[start..start+len].chars().count() - 1;
                ans += &string_to.clone();
            }else{
                ans += &ch.to_string();
            }
            start += ch.len_utf8();
        }
    
        *text = ans;
        true
    }
}
