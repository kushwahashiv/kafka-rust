use crate::db::schema::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Identifiable, Queryable, Insertable, Associations, AsChangeset)]
#[primary_key(balance_id)]
#[table_name = "balance"]
pub struct Balance {
    pub balance_id: i32,
    pub iban: String,
    pub token: String,
    pub amount: i64,
    pub type_: String,
    pub lmt: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

#[derive(Debug, PartialEq, Queryable, Insertable, Associations, AsChangeset)]
#[table_name = "balance"]
pub struct NewBalance<'a> {
    pub iban: &'a str,
    pub token: &'a str,
    pub amount: i64,
    pub type_: &'a str,
    pub lmt: i64
}

#[derive(Clone, Debug, Queryable, Deserialize, Serialize)]
pub struct Cac {
    pub uuid: String,
    pub iban: Option<String>,
    pub token: Option<String>,
    pub type_: Option<String>,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime
}

#[derive(Debug, PartialEq, Queryable, Insertable, Associations, AsChangeset)]
#[table_name = "cacr"]
pub struct NewCac<'a> {
    pub uuid: String,
    pub iban: Option<&'a str>,
    pub token: Option<&'a str>,
    pub type_: Option<&'a str>,
    pub reason: Option<&'a str>
}

#[derive(Clone, Debug, Queryable, Deserialize, Serialize)]
pub struct Cmt {
    pub uuid: String,
    pub reason: Option<String>,
    pub created_at: NaiveDateTime
}

#[derive(Debug, PartialEq, Queryable, Insertable, Associations, AsChangeset)]
#[table_name = "cmtr"]
pub struct NewCmt<'a> {
    pub uuid: String,
    pub reason: Option<&'a str>
}
