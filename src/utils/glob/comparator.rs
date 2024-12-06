//SPDX-FileCopyrightText: 2024 Ryuichi Ueda <ryuichiueda@gmail.com>
//SPDX-License-Identifier: BSD-3-Clause

use super::GlobElem;

pub fn shave_word(word: &String, pattern: &Vec<GlobElem>) -> Vec<String> {
    let mut candidates = vec![word.to_string()];
    pattern.iter().for_each(|w| shave(&mut candidates, &w) );
    candidates
}

pub fn shave(candidates: &mut Vec<String>, w: &GlobElem) {
    match w {
        GlobElem::Normal(s) => string(candidates, &s),
        GlobElem::Symbol('?') => question(candidates),
        _ => panic!("Unknown glob symbol"),
    }
}

fn string(cands: &mut Vec<String>, s: &String) {
    cands.retain(|c| c.starts_with(s) );
    cands.iter_mut().for_each(|c| {*c = c.split_off(s.len());});
}

fn question(cands: &mut Vec<String>) {
    cands.retain(|c| c.len() != 0 );
    let len = |c: &String| c.chars().nth(0).unwrap().len_utf8();
    cands.iter_mut().for_each(|c| {*c = c.split_off(len(c));});
}
