use chrono::{NaiveDateTime, Utc};
use diesel::pg::PgConnection;

use crate::db::schema::*;
use crate::db::util::*;
use avro_rs::types::Value;
use diesel::{self, prelude::*};
use log::warn;

#[derive(Debug, PartialEq, Identifiable, Queryable, Insertable, Associations, AsChangeset)]
#[primary_key(id)]
#[table_name = "balance"]
pub struct Balance {
    pub id: String,
    pub account_no: String,
    pub amount: f64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

impl Balance {
    pub fn new(conn: &PgConnection, account_no: String) -> Self {
        let now = Utc::now().naive_utc();

        let new_balance = Self {
            id: get_id(),
            account_no: account_no,
            amount: 0.0,
            updated_at: now,
            created_at: now
        };

        diesel::insert_into(balance::table)
            .values(&new_balance)
            .get_result(conn)
            .expect("Error saving new balance")
    }

    pub fn get_balance_by_account_no(conn: &PgConnection, account_no: &str) -> Option<Balance> {
        balance::table
            .filter(balance::account_no.eq(account_no))
            .first::<Balance>(conn)
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
    pub account_type: String,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

impl ConfirmedAccount {
    pub fn new(id: String, account_no: String) -> Self {
        let now = Utc::now().naive_utc();

        Self {
            id: id,
            account_no: account_no,
            account_type: String::from("Basic"),
            reason: None,
            updated_at: now,
            created_at: now
        }
    }

    pub fn get_account_by_id(conn: &PgConnection, id: &str) -> Option<ConfirmedAccount> {
        confirmed_account::table
            .filter(confirmed_account::id.eq(id))
            .first::<ConfirmedAccount>(conn)
            .optional()
            .unwrap()
    }

    pub fn get_confirmed_account(conn: &PgConnection, id: String) -> ConfirmedAccount {
        match confirmed_account::table.find(id.clone()).first::<ConfirmedAccount>(conn).optional() {
            Ok(Some(v)) => v,
            Ok(None) => ConfirmedAccount::create_account(id, conn),
            Err(e) => panic!("Error trying to get confirmed ccount creation with id: {:?} and error: {}", id, e)
        }
    }

    pub fn create_account(id: String, conn: &PgConnection) -> ConfirmedAccount {
        let account = new_account();
        let reason = match Balance::get_balance_by_account_no(conn, &account) {
            Some(_v) => Option::from("generated account no already exists, try again"),
            None => None
        };

        if reason == None {
            Balance::new(conn, account.clone());
        };

        let new_account = ConfirmedAccount::new(id, account.clone());

        diesel::insert_into(confirmed_account::table)
            .values(&new_account)
            .get_result(conn)
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
    pub fn new(id: String, reason: Option<&str>) -> Self {
        let now = Utc::now().naive_utc();

        Self {
            id: id,
            reason: reason.map(|s| s.to_string()),
            created_at: now
        }
    }

    pub fn get_confirmed_transaction(conn: &PgConnection, id: String, values: &[(String, Value)]) -> (ConfirmedTransaction, Option<Balance>, Option<Balance>) {
        match confirmed_transaction::table.find(id.clone()).first::<ConfirmedTransaction>(conn).optional() {
            Ok(Some(v)) => (v, None, None),
            Ok(None) => (ConfirmedTransaction::create_money_transaction(conn, id, values)),
            Err(e) => panic!("Error trying to get confirmed account with id: {:?} and error: {}", id, e)
        }
    }

    pub fn create_money_transaction(conn: &PgConnection, id: String, values: &[(String, Value)]) -> (ConfirmedTransaction, Option<Balance>, Option<Balance>) {
        let from = match values[3] {
            (ref _from, Value::String(ref v)) => v,
            _ => panic!("Not a string value, while that was expected")
        };
        let to = match values[4] {
            (ref _to, Value::String(ref v)) => v,
            _ => panic!("Not a string value, while that was expected")
        };

        let (reason, b_from, b_to) = if invalid_from(from) {
            (Option::from("from is invalid"), None, None)
        } else if from == to {
            (Option::from("from and to can't be same for transfer"), None, None)
        } else {
            ConfirmedTransaction::transfer(conn, values, from, to)
        };

        let new_confirmed_account = ConfirmedTransaction::new(id, reason);
        let cmt = diesel::insert_into(confirmed_transaction::table)
            .values(&new_confirmed_account)
            .get_result(conn)
            .expect("Error saving new balance");
        (cmt, b_from, b_to)
    }

    fn transfer(conn: &PgConnection, values: &[(String, Value)], from: &str, to: &str) -> (Option<&'static str>, Option<Balance>, Option<Balance>) {
        let am = match values[2] {
            (ref _amount, Value::Double(ref v)) => v,
            _ => panic!("Not a Long value, while that was expected")
        };
        let (reason, b_from) = if valid_open_account(from) {
            match Balance::get_balance_by_account_no(conn, from) {
                Some(v) => {
                    let b_from = match diesel::update(&v).set(balance::amount.eq(balance::amount - am)).get_result::<Balance>(conn) {
                        Ok(v) => Option::from(v),
                        Err(e) => panic!("error updating balance with account no: {}, error: {}", to, e)
                    };
                    (None, b_from)
                }
                None => {
                    warn!("Valid open account no {} not found", from);
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        let b_to = match reason {
            None => {
                if valid_open_account(to) {
                    match Balance::get_balance_by_account_no(conn, to) {
                        Some(v) => match diesel::update(&v).set(balance::amount.eq(balance::amount + am)).get_result::<Balance>(conn) {
                            Ok(v) => Option::from(v),
                            Err(e) => panic!("error updating balance with account no: {}, error: {}", to, e)
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
