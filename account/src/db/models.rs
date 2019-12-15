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
    pub amount: f64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

impl Balance {
    pub fn new() -> Self {
        let now = Utc::now().naive_utc();

        Self {
            id: get_id(),
            amount: 5000.0,
            updated_at: now,
            created_at: now
        }
    }

    pub fn get_balance_by_id(conn: &PgConnection, id: &str) -> Option<Balance> {
        balance::table.filter(balance::id.eq(id)).first::<Balance>(conn).optional().unwrap()
    }

    pub fn transfer_money(conn: &PgConnection, uuid: String, values: &[(String, Value)]) -> (Account, Option<Balance>, Option<Balance>) {
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

        let new_balance = Account::new();

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
        let b_from = match Balance::get_balance_by_id(conn, from) {
            Some(v) => match diesel::update(&v).set(balance::amount.eq(balance::amount - am)).get_result::<Balance>(conn) {
                Ok(v) => Option::from(v),
                Err(e) => panic!("error updating balance with iban: {}, error: {}", to, e)
            },

            None => {
                warn!("Valid open iban {} not found", from);
                None
            }
        };
        let b_to = match Balance::get_balance_by_id(conn, to) {
            Some(v) => match diesel::update(&v).set(balance::amount.eq(balance::amount + am)).get_result::<Balance>(conn) {
                Ok(v) => Option::from(v),
                Err(e) => panic!("error updating balance with iban: {}, error: {}", to, e)
            },
            None => {
                warn!("Valid open iban {} not found", from);
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
    pub fn new() -> Self {
        let now = Utc::now().naive_utc();

        Self {
            id: get_id(),
            account_no: String::new(),
            account_type: String::from("Basic"),
            updated_at: now,
            created_at: now
        }
    }

    pub fn get_account_by_id(conn: &PgConnection, id: &str) -> Option<Account> {
        account::table.filter(account::id.eq(id)).first::<Account>(conn).optional().unwrap()
    }

    pub fn create_account(conn: &PgConnection) -> Account {
        let new_account = Account::new();
        diesel::insert_into(account::table)
            .values(&new_account)
            .get_result(conn)
            .expect("Error saving new account")
    }
}
