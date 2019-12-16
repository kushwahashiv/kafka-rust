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

    pub fn get_money_transfer(conn: &PgConnection, id: String, values: &[(String, Value)]) -> (Account, Option<Balance>, Option<Balance>) {
        match account::table.find(id.clone()).first::<Account>(conn).optional() {
            Ok(Some(v)) => (v, None, None),
            Ok(None) => (Balance::create_transfer_money(conn, id, values)),
            Err(e) => panic!("Error trying to get account with id: {:?} and error: {}", id, e)
        }
    }

    pub fn create_transfer_money(conn: &PgConnection, id: String, values: &[(String, Value)]) -> (Account, Option<Balance>, Option<Balance>) {
        let from = match values[3] {
            (ref _from, Value::String(ref v)) => v,
            _ => panic!("Not a string value, while that was expected")
        };
        let to = match values[4] {
            (ref _to, Value::String(ref v)) => v,
            _ => panic!("Not a string value, while that was expected")
        };

        let (b_from, b_to) = if from == to {
            (None, None)
        } else {
            Balance::transfer(conn, values, from, to)
        };

        let new_balance = Account::new(id, String::from(""));

        //TODO: update balance
        let account = diesel::insert_into(account::table)
            .values(&new_balance)
            .get_result(conn)
            .expect("Error saving new balance");
        (account, b_from, b_to)
    }

    fn transfer(conn: &PgConnection, values: &[(String, Value)], from: &str, to: &str) -> (Option<Balance>, Option<Balance>) {
        let am = match values[2] {
            (ref _amount, Value::Double(ref v)) => v,
            _ => panic!("Not a Long value, while that was expected")
        };
        let b_from = match Balance::get_balance_by_account_no(conn, from) {
            Some(v) => match diesel::update(&v).set(balance::amount.eq(balance::amount - am)).get_result::<Balance>(conn) {
                Ok(v) => Option::from(v),
                Err(e) => panic!("error updating balance with account no: {}, error: {}", to, e)
            },

            None => {
                warn!("Valid open account no {} not found", from);
                None
            }
        };
        let b_to = match Balance::get_balance_by_account_no(conn, to) {
            Some(v) => match diesel::update(&v).set(balance::amount.eq(balance::amount + am)).get_result::<Balance>(conn) {
                Ok(v) => Option::from(v),
                Err(e) => panic!("error updating balance with account no: {}, error: {}", to, e)
            },
            None => {
                warn!("Valid open account no {} not found", from);
                None
            }
        };
        (b_from, b_to)
    }
}

#[derive(Debug, PartialEq, Identifiable, Queryable, Insertable, Associations, AsChangeset)]
#[primary_key(id)]
#[table_name = "account"]
pub struct Account {
    pub id: String,
    pub account_no: String,
    pub account_type: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

impl Account {
    pub fn new(id: String, account_no: String) -> Self {
        let now = Utc::now().naive_utc();

        Self {
            id: id,
            account_no: account_no,
            account_type: String::from("Basic"),
            updated_at: now,
            created_at: now
        }
    }

    pub fn get_account_by_id(conn: &PgConnection, id: &str) -> Option<Account> {
        account::table.filter(account::id.eq(id)).first::<Account>(conn).optional().unwrap()
    }

    pub fn get_confirm_account(conn: &PgConnection, id: String) -> Account {
        match account::table.find(id.clone()).first::<Account>(conn).optional() {
            Ok(Some(v)) => v,
            Ok(None) => Account::create_account(id, conn),
            Err(e) => panic!("Error trying to get confirm account creation with id: {:?} and error: {}", id, e)
        }
    }

    pub fn create_account(id: String, conn: &PgConnection) -> Account {
        let account = new_account();
        let reason = match Balance::get_balance_by_account_no(conn, &account) {
            Some(_v) => Option::from("generated account no already exists, try again"),
            None => None
        };

        if reason == None {
            Balance::new(conn, account.clone());
        };

        let new_account = Account::new(id, account.clone());

        diesel::insert_into(account::table)
            .values(&new_account)
            .get_result(conn)
            .expect("Error saving new account")
    }
}
