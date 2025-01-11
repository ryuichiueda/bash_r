//SPDXFileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDXLicense-Identifier: BSD-3-Clause

use crate::core::DataBase;

pub fn special_param(db :&DataBase, name: &str) -> Option<String> {
    let val = match name {
        "-" => db.flags.clone(),
        "?" => db.exit_status.to_string(),
        "_" => db.last_arg.clone(),
        "#" => {
            let pos = db.position_parameters.len() - 1;
            (db.position_parameters[pos].len() - 1).to_string()
        },
        _ => return None,
    };

    Some(val)
}

pub fn connected_position_params(db :&DataBase) -> Result<String, String> {
    match db.position_parameters.last() {
        Some(a) => Ok(a[1..].join(" ")),
        _       => Ok("".to_string()),
    }
}

/*
pub fn position_param_pos(&self, key: &str) -> Option<usize> {
    if ! (key.len() == 1 && "0" <= key && key <= "9") {
        return None;
    }

    let n = key.parse::<usize>().unwrap();
    let layer = self.position_parameters.len();
    match n < self.position_parameters[layer-1].len() {
        true  => Some(n),
        false => None,
    }
}
*/

pub fn position_param(db: &DataBase, pos: usize) -> Result<String, String> {
    let layer = db.position_parameters.len();
    return match db.position_parameters[layer-1].len() > pos {
        true  => Ok(db.position_parameters[layer-1][pos].to_string()),
        false => Ok(String::new()),
    };
}
