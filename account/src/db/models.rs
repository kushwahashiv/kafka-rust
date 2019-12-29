use crate::db::schema::*;
use crate::db::util::*;
use crate::db::DbConn;
use avro_rs::types::Value;
use chrono::{NaiveDateTime, Utc};
use diesel::{self, prelude::*};
use log::warn;

#[derive(Debug, PartialEq, Identifiable, Queryable, Insertable, Associations, AsChangeset)]
#[primary_key(id)]
#[table_name = "balance"]
pub struct Balance {
    pub id: String,
    pub account_no: String,
    pub token: String,
    pub account_type: String,
    pub amount: f64,
    pub limits: f64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

impl Balance {
    pub fn new(account_no: String, token: String, tp: String, conn: &DbConn) -> () {
        let now = Utc::now().naive_utc();

        let new_balance = Self {
            id: get_id(),
            account_no: account_no,
            token: token,
            account_type: tp,
            amount: 0.0,
            limits: -50000.0,
            updated_at: now,
            created_at: now
        };

        diesel::insert_into(balance::table)
            .values(new_balance)
            .execute(&**conn)
            .expect("Error saving new balance");
    }

    pub fn get_balance_by_account_no(account_no: String, conn: &DbConn) -> Option<Balance> {
        balance::table
            .filter(balance::account_no.eq(account_no))
            .first::<Balance>(&**conn)
            .optional()
            .unwrap()
    }
}

#[derive(Debug, PartialEq, Identifiable, Queryable, Insertable, Associations, AsChangeset)]
#[primary_key(id)]
#[table_name = "confirmed_account"]
pub struct ConfirmedAccount {
    pub id: String,
    pub account_no: String,
    pub token: String,
    pub account_type: String,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

impl ConfirmedAccount {
    pub fn new(id: String, account_no: String, token: String, tp: String, reason: Option<String>) -> Self {
        let now = Utc::now().naive_utc();

        Self {
            id: id,
            account_no: account_no,
            token: token,
            account_type: tp,
            reason: reason,
            updated_at: now,
            created_at: now
        }
    }

    pub fn get_account_by_id(id: String, conn: &DbConn) -> Option<ConfirmedAccount> {
        confirmed_account::table
            .filter(confirmed_account::id.eq(id))
            .first::<ConfirmedAccount>(&**conn)
            .optional()
            .unwrap()
    }

    pub fn get_cac(id: String, tp: String, conn: &DbConn) -> ConfirmedAccount {
        match confirmed_account::table.find(id.clone()).first::<ConfirmedAccount>(&**conn).optional() {
            Ok(Some(v)) => v,
            Ok(None) => ConfirmedAccount::create_cac(id, tp, conn),
            Err(e) => panic!("Error trying to get confirmed ccount creation with id: {:?} and error: {}", id, e)
        }
    }

    pub fn create_cac(id: String, tp: String, conn: &DbConn) -> ConfirmedAccount {
        let account_no = new_account();
        let reason = match Balance::get_balance_by_account_no(account_no.clone(), &conn) {
            Some(_v) => Option::from("generated account no already exists, try again"),
            None => None
        };
        let token = match reason {
            Some(_v) => String::new(),
            None => new_token()
        };

        if reason == None {
            Balance::new(account_no.clone(), token.clone(), tp.clone(), conn);
        };

        let new_cac = ConfirmedAccount::new(id, account_no.clone(), token.clone(), tp.clone(), Option::from(reason.map(|s| s.to_string())));

        diesel::insert_into(confirmed_account::table)
            .values(&new_cac)
            .get_result(&**conn)
            .expect("Error saving new account")
    }
}

#[derive(Debug, PartialEq, Identifiable, Queryable, Insertable, Associations, AsChangeset)]
#[primary_key(id)]
#[table_name = "confirmed_transaction"]
pub struct ConfirmedTransaction {
    pub id: String,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime
}

impl ConfirmedTransaction {
    pub fn new(id: String, reason: Option<String>) -> Self {
        let now = Utc::now().naive_utc();

        Self {
            id: id,
            reason: reason, // reason.map(|s| s.to_string()),
            created_at: now
        }
    }

    pub fn get_cmt(id: String, values: &[(String, Value)], conn: &DbConn) -> (ConfirmedTransaction, Option<Balance>, Option<Balance>) {
        match confirmed_transaction::table.find(id.clone()).first::<ConfirmedTransaction>(&**conn).optional() {
            Ok(Some(v)) => (v, None, None),
            Ok(None) => (ConfirmedTransaction::create_cmt(id, values, conn)),
            Err(e) => panic!("Error trying to get confirmed account with id: {:?} and error: {}", id, e)
        }
    }

    pub fn create_cmt(id: String, values: &[(String, Value)], conn: &DbConn) -> (ConfirmedTransaction, Option<Balance>, Option<Balance>) {
        let from = match values[3] {
            (ref _from, Value::String(ref v)) => v.clone(),
            _ => panic!("Not a string value, while that was expected")
        };
        let to = match values[4] {
            (ref _to, Value::String(ref v)) => v.clone(),
            _ => panic!("Not a string value, while that was expected")
        };

        let (reason, b_from, b_to) = if invalid_from(from.clone()) {
            (Option::from("from is invalid"), None, None)
        } else if from == to {
            (Option::from("from and to can't be same for transfer"), None, None)
        } else {
            ConfirmedTransaction::transfer(values, from, to, conn)
        };

        let new_confirmed_account = ConfirmedTransaction::new(id, reason.map(|s| s.to_string()));
        let cmt = diesel::insert_into(confirmed_transaction::table)
            .values(&new_confirmed_account)
            .get_result(&**conn)
            .expect("Error saving new balance");
        (cmt, b_from, b_to)
    }

    fn transfer(values: &[(String, Value)], from: String, to: String, conn: &DbConn) -> (Option<&'static str>, Option<Balance>, Option<Balance>) {
        let am = match values[2] {
            (ref _amount, Value::Double(ref v)) => v,
            _ => panic!("Not a Long value, while that was expected")
        };
        let (reason, b_from) = if valid_open_account(from.clone()) {
            match Balance::get_balance_by_account_no(from.clone(), &conn) {
                Some(v) => {
                    let b_from = match diesel::update(&v).set(balance::amount.eq(balance::amount - am)).get_result::<Balance>(&**conn) {
                        Ok(v) => Option::from(v),
                        Err(e) => panic!("error updating balance with account no: {}, error: {}", to, e)
                    };
                    (None, b_from)
                }
                None => {
                    warn!("Valid open account no {} not found", from.clone());
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        let b_to = match reason {
            None => {
                if valid_open_account(to.clone()) {
                    match Balance::get_balance_by_account_no(to.clone(), conn) {
                        Some(v) => match diesel::update(&v).set(balance::amount.eq(balance::amount + am)).get_result::<Balance>(&**conn) {
                            Ok(v) => Option::from(v),
                            Err(e) => panic!("error updating balance with account no: {}, error: {}", to.clone(), e)
                        },
                        None => {
                            warn!("Valid open account no {} not found", from);
                            None
                        }
                    }
                } else {
                    None
                }
            }
            Some(_) => None
        };
        (reason, b_from, b_to)
    }
}
