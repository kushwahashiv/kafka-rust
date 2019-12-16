use rand::prelude::*;
use std::str::FromStr;
use std::u128;

pub fn get_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn new_account() -> String {
    let mut rng = thread_rng();
    let mut digits = String::from("0");
    for _i in 0..=8 {
        digits.push_str(rng.gen_range(0, 10).to_string().as_ref());
    }
    open_account(&digits[..])
}

fn open_account(digits: &str) -> String {
    let check_nr = get_check_nr(digits);
    format!("NL{}OPEN{}", check_nr, digits)
}

fn get_check_nr(digits: &str) -> String {
    let string_for_check = format!("24251423{}232100", digits);
    let check_nr = (98 - u128::from_str(string_for_check.as_ref()).unwrap() % 97).to_string();
    if check_nr.len() == 1 {
        format!("0{}", check_nr)
    } else {
        check_nr
    }
}

pub fn invalid_from(from: String) -> bool {
    if "cash" == from {
        false
    } else {
        !valid_open_account(from)
    }
}

pub fn valid_open_account(account: String) -> bool {
    if account.len() != 18 {
        false
    } else {
        account == open_account(&account[8..])
    }
}
