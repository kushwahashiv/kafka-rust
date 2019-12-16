extern crate openssl;
#[macro_use]
extern crate diesel;
extern crate log;
extern crate rocket;
extern crate serde_json;
use dotenv::dotenv;
use std::env;

mod db;
mod kafka_consumer;
mod kafka_producer;
mod logger;

use crate::db::models::{Balance, Cac};

use crate::db::Pool;
use crate::kafka_consumer::{consume, ValuesProcessor};
use crate::kafka_producer::get_producer;
use crate::logger::setup_logger;
use avro_rs::types::Value;
use chrono::Utc;
use diesel::pg::PgConnection;
use log::{error, info};
use schema_registry_converter::schema_registry::SubjectNameStrategy;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

struct CacContext {
    sender: Sender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for CacContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_cac(values, &self.pool.get().unwrap(), &self.sender)
    }
}

struct ProducerData {
    topic: &'static str,
    key: String,
    values: Vec<(&'static str, Value)>
}

fn handle_cac(values: &[(String, Value)], conn: &PgConnection, sender: &Sender<ProducerData>) {
    let uuid = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not a fixed value of 16, while that was expected")
    };
    let type_ = match &values[1] {
        (_a_type, Value::Enum(_index, v)) => v,
        _ => panic!("Not an enum, while that was expected")
    };
    let cac = db::get_cac(conn, uuid.clone(), type_);
    let key = uuid;
    let producer_data = match cac.reason {
        None => ProducerData {
            topic: "account_creation_confirmed",
            key,
            values: acc_vec(&values, cac)
        },
        Some(v) => ProducerData {
            topic: "account_creation_failed",
            key,
            values: fail_vec(&values, v)
        }
    };
    sender.send(producer_data).unwrap();
}

fn acc_vec(cac_values: &[(String, Value)], cac: Cac) -> Vec<(&'static str, Value)> {
    let id = match cac_values[0] {
        (ref _id, Value::String(ref v)) => ("id", Value::String(v.clone())),
        _ => panic!("Not a fixed value of 16, while that was expected")
    };
    let iban = match cac.iban {
        Some(v) => ("iban", Value::String(v)),
        None => panic!("No iban present in cac while expected because sending acc")
    };
    let token = match cac.token {
        Some(v) => ("token", Value::String(v)),
        None => panic!("No token present in cac while expected because sending acc")
    };
    let tp = match cac_values[1] {
        (ref _a_type, Value::Enum(ref i, ref v)) => ("a_type", Value::Enum(*i, v.clone())),
        _ => panic!("Not a fixed value of 16, while that was expected")
    };
    vec![id, iban, token, tp]
}

fn fail_vec(cac_values: &[(String, Value)], reason: String) -> Vec<(&'static str, Value)> {
    let id = match cac_values[0] {
        (ref _id, Value::String(ref v)) => ("id", Value::String(v.clone())),
        _ => panic!("Not a fixed value of 16, while that was expected")
    };
    vec![id, ("reason", Value::String(reason))]
}

struct CmtContext {
    sender: Sender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for CmtContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_cmt(values, &self.pool.get().unwrap(), &self.sender)
    }
}

fn handle_cmt(values: &[(String, Value)], conn: &PgConnection, sender: &Sender<ProducerData>) {
    let uuid = match &values[0] {
        (_id, Value::String(v)) => v.clone(),
        _ => panic!("Not a fixed value of 16, while that was expected")
    };
    let (cmt, b_from, b_to) = db::get_cmt(conn, uuid.clone(), values);
    let key = uuid;
    {
        let producer_data = match cmt.reason {
            None => ProducerData {
                topic: "money_transfer_confirmed",
                key,
                values: mtc_vec(values)
            },
            Some(v) => ProducerData {
                topic: "money_transfer_failed",
                key,
                values: fail_vec(values, v)
            }
        };
        sender.send(producer_data).unwrap();
    }
    match b_from {
        None => info!("No balance -from- present, no balance_changed send"),
        Some(v) => send_bc(true, &values, v, sender)
    }
    match b_to {
        None => info!("No balance -to- present, no balance_changed send"),
        Some(v) => send_bc(false, &values, v, sender)
    }
}

fn mtc_vec(cmt_values: &[(String, Value)]) -> Vec<(&'static str, Value)> {
    let id = match cmt_values[0] {
        (ref _id, Value::String(ref v)) => ("id", Value::String(v.clone())),
        _ => panic!("Not a fixed value of 16, while that was expected")
    };
    vec![id]
}

fn send_bc(is_from: bool, cmt_values: &[(String, Value)], balance: Balance, sender: &Sender<ProducerData>) {
    let iban_string = balance.iban;
    let iban = ("iban", Value::String(iban_string.clone()));
    let amount = ("new_balance", Value::Long(balance.amount));
    let changed_by = match cmt_values[2] {
        (ref _amount, Value::Long(ref v)) => {
            if is_from {
                ("changed_by", Value::Long(-*v))
            } else {
                ("changed_by", Value::Long(*v))
            }
        }
        _ => panic!("Not a Long value, while that was expected")
    };
    let from_to = match cmt_values[if is_from { 4 } else { 3 }] {
        (ref _from, Value::String(ref v)) => ("from_to", Value::String(v.clone())),
        _ => panic!("Not a string value, while that was expected")
    };
    let description = match cmt_values[5] {
        (ref _description, Value::String(ref v)) => ("description", Value::String(v.clone())),
        _ => panic!("Not a string value, while that was expected")
    };
    let producer_data = ProducerData {
        topic: "balance_changed",
        key: iban_string,
        values: vec![iban, amount, changed_by, from_to, description]
    };
    sender.send(producer_data).unwrap();
}

use rocket::handler::Outcome;
use rocket::http::Method::*;
use rocket::{Data, Request, Route};

fn cac<'r>(req: &'r Request, _data: Data) -> Outcome<'r> {
    let uuid = String::from("83cd1a69-7b8d-4496-a19e-003697a7281b"); // get_id();
    let id = (String::from("uuid"), Value::String(uuid.clone()));
    let type_ = (String::from("type_"), Value::Enum(0, String::from("AUTO")));

    let data = vec![id, type_];
    let (sender, receiver) = mpsc::channel();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::connect(&database_url);
    handle_cac(&data[..], &pool.get().unwrap(), &sender);

    Outcome::from(req, String::from("ss"))
}

fn cmt<'r>(req: &'r Request, _data: Data) -> Outcome<'r> {
    let now = Utc::now().naive_utc();
    let uuid = String::from("83cd1a69-7b8d-4496-a19e-003697a7281b"); // get_id();
    let id = (String::from("uuid"), Value::String(uuid.clone()));
    let reason = (String::from("reason"), Value::String(String::from("cac")));
    let created_at = (String::from("created_at"), Value::String(now.to_string()));
    let from = (String::from("from"), Value::String(String::from("from")));
    let to = (String::from("to"), Value::String(String::from("to")));

    let data = vec![id, reason, created_at, from, to];
    let (sender, receiver) = mpsc::channel();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::connect(&database_url);
    handle_cmt(&data[..], &pool.get().unwrap(), &sender);

    Outcome::from(req, String::from("data"))
}

fn launch_rocket(p: Pool) {
    let rocket = rocket::ignite();

    let get_cmt = Route::new(Get, "/", cmt);
    let get_cac = Route::new(Get, "/", cac);
    let rocket = rocket.mount("/cac", vec![get_cac]).mount("/cmt", vec![get_cmt]);

    let rocket = rocket.manage(p);
    log::set_max_level(log::LevelFilter::max());
    error!("Launch error {:#?}", rocket.launch());
}

#[macro_use]
extern crate diesel_migrations;

embed_migrations!("./migrations");

#[allow(unused_imports)]
mod migrations {
    embed_migrations!();
    pub fn run_migrations(p: crate::db::Pool) {
        use std::io::stdout;
        embedded_migrations::run_with_output(&*p.get().expect("connection instance"), &mut stdout()).expect("Can't run migrations");
    }
}

fn main() {
    setup_logger(None);
    let group_id = "transaction";
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || send_loop(&receiver));

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::connect(&database_url);
    migrations::run_migrations(pool.clone());
    launch_rocket(pool.clone());

    let cac_handle = consume(
        group_id,
        "confirm_account_creation",
        Box::from(CacContext {
            sender: sender.clone(),
            pool: pool.clone()
        })
    );
    let cmt_handle = consume(
        group_id,
        "confirm_money_transfer",
        Box::from(CmtContext {
            sender: sender.clone(),
            pool: pool.clone()
        })
    );
    cac_handle.join().expect_err("Error closing cac handler");
    cmt_handle.join().expect_err("Error closing cmt handler");
}

fn send_loop(receiver: &Receiver<ProducerData>) {
    let mut producer = get_producer();
    let mut cache = HashMap::new();
    loop {
        let producer_data = match receiver.recv() {
            Ok(v) => v,
            Err(e) => panic!("Error reading future from receiver: {}", e)
        };
        let strategy = cache
            .entry(producer_data.topic)
            .or_insert_with(|| SubjectNameStrategy::TopicNameStrategy(producer_data.topic.into(), false));
        producer.send(producer_data.topic, producer_data.key, producer_data.values, strategy);
    }
}
