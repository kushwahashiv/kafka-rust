use chrono::{NaiveDateTime, Utc};

use crate::db::schema::*;
use crate::db::util::*;
use crate::db::DbConn;
// use avro_rs::types::Value;
use diesel::{self, prelude::*};

#[derive(Debug, PartialEq, Identifiable, Queryable, Insertable, Associations, AsChangeset)]
#[primary_key(id)]
#[table_name = "transactions"]
pub struct Transactions {
    pub id: String,
    pub account_no: String,
    pub amount: f64,
    pub new_balance: f64,
    pub account_type: String,
    pub changed_by: String,
    pub from_to: String,
    pub direction: String,
    pub description: String,
    pub created_at: NaiveDateTime
}

impl Transactions {
    pub fn new(account_no: String) -> Self {
        let now = Utc::now().naive_utc();

        Self {
            id: get_id(),
            account_no: account_no,
            amount: 0.0,
            new_balance: 0.0,
            account_type: String::new(),
            changed_by: String::new(),
            from_to: String::new(),
            direction: String::new(),
            description: String::new(),
            created_at: now
        }
    }

    pub fn find_transaction_by_id(id: &str, conn: &DbConn) -> Option<Transactions> {
        transactions::table
            .filter(transactions::id.eq(id))
            .first::<Transactions>(&**conn)
            .optional()
            .unwrap()
    }

    pub fn find_transactions_by_account_no(account_no: &str, conn: &DbConn) -> Option<Transactions> {
        transactions::table
            .filter(transactions::account_no.eq(account_no))
            .first::<Transactions>(&**conn)
            .optional()
            .unwrap()
    }

    pub fn find_all_last_transactions(conn: &DbConn) -> Option<Vec<Transactions>> {
        transactions::table.load::<Transactions>(&**conn).optional().unwrap()
    }
}

#[derive(Debug, Serialize, PartialEq, Identifiable, Queryable, Insertable, Associations, AsChangeset)]
#[primary_key(id)]
#[table_name = "account"]
pub struct Account {
    pub id: String,
    pub username: String,
    pub password: String,
    pub created_at: NaiveDateTime
}

impl Account {
    pub fn new(username: String, password: String) -> Self {
        let now = Utc::now().naive_utc();

        Self {
            id: get_id(),
            username: username,
            password: password,
            created_at: now
        }
    }

    pub fn find_account_by_username(username: &str, conn: &DbConn) -> Option<Account> {
        account::table
            .filter(account::username.eq(username))
            .first::<Account>(&**conn)
            .optional()
            .unwrap()
    }

    pub fn insert_account(acc: Account, conn: &DbConn) -> Account {
        diesel::insert_into(account::table)
            .values(&acc)
            .get_result(&**conn)
            .expect("Error saving new account")
    }

    pub fn remove_account(id: String, conn: &DbConn) -> () {
        diesel::delete(account::table.filter(account::id.eq(id)))
            .execute(&**conn)
            .expect("Error deleting account");
    }

    pub fn get_account(username: String, password: String, conn: &DbConn) -> Account {
        match Account::find_account_by_username(&username, conn) {
            Some(v) => v,
            None => Account::insert_account(Account::new(username, password), &conn)
        }
    }
}

// create-money-transfer
// money-transfer
