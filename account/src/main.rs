#![feature(proc_macro_hygiene, decl_macro, vec_remove_item, try_trait)]
#![recursion_limit = "512"]

extern crate openssl;
#[macro_use]
extern crate diesel;
extern crate log;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use dotenv::dotenv;
use std::env;

mod db;
mod kafka_consumer;
mod kafka_producer;
mod logger;

use crate::db::models::{Balance, ConfirmedAccount};

use crate::db::DbConn;
use crate::db::Pool;
use crate::kafka_consumer::{consume, ValuesProcessor};
use crate::kafka_producer::get_producer;
use crate::logger::setup_logger;
use avro_rs::types::Value;
use log::{error, info};
use rocket::config::{Config, Environment, LoggingLevel};
use rocket_contrib::json::Json;
use schema_registry_converter::schema_registry::SubjectNameStrategy;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;

struct CacContext {
    sender: SyncSender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for CacContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_cac(values, &DbConn(self.pool.get().unwrap()), &self.sender)
    }
}

struct ProducerData {
    topic: &'static str,
    key: String,
    values: Vec<(&'static str, Value)>
}

fn handle_cac(values: &[(String, Value)], conn: &DbConn, sender: &SyncSender<ProducerData>) {
    let id = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an id, while that was expected")
    };
    let _type = match &values[1] {
        (_type, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an enum, while that was expected")
    };
    let cac = db::ConfirmedAccount::get_cac(id.clone(), _type, conn);
    let key = id.clone();
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

fn acc_vec(cac_values: &[(String, Value)], cac: ConfirmedAccount) -> Vec<(&'static str, Value)> {
    let id = match cac_values[0] {
        (ref _id, Value::String(ref v)) => ("id", Value::String(v.clone())),
        _ => panic!("Not an id, while that was expected")
    };
    let account_no = ("account_no", Value::String(cac.account_no));
    let token = ("token", Value::String(cac.token));

    let tp = match cac_values[1] {
        (ref _type, Value::String(ref v)) => ("_type", Value::String(v.clone())),
        _ => panic!("Not an type, while that was expected")
    };

    vec![id, account_no, token, tp]
}

fn fail_vec(cac_values: &[(String, Value)], reason: String) -> Vec<(&'static str, Value)> {
    let id = match cac_values[0] {
        (ref _id, Value::String(ref v)) => ("id", Value::String(v.clone())),
        _ => panic!("Not an id, while that was expected")
    };
    vec![id, ("reason", Value::String(reason))]
}

struct CmtContext {
    sender: SyncSender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for CmtContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_cmt(values, &DbConn(self.pool.get().unwrap()), &self.sender)
    }
}

fn handle_cmt(values: &[(String, Value)], conn: &DbConn, sender: &SyncSender<ProducerData>) {
    let id = match &values[0] {
        (_id, Value::String(v)) => v.clone(),
        _ => panic!("Not a an id, while that was expected")
    };
    let (cmt, b_from, b_to) = db::ConfirmedTransaction::get_cmt(id.clone(), values, conn);
    let key = id;
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

fn send_bc(is_from: bool, cmt_values: &[(String, Value)], balance: Balance, sender: &SyncSender<ProducerData>) {
    let account_string = balance.account_no;
    let account_no = ("account_no", Value::String(account_string.clone()));
    let amount = ("new_balance", Value::Double(balance.amount));
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
        key: account_string,
        values: vec![account_no, amount, changed_by, from_to, description]
    };
    sender.send(producer_data).unwrap();
}

#[derive(Deserialize, Serialize)]
#[allow(non_snake_case)]
struct LoginData {
    username: String,
    password: String
}

#[get("/cac")]
fn cac(conn: DbConn, sender: State<JobSender>) -> Json<String> {
    Json(String::from("acc"))
}

#[get("/cmt")]
fn cmt(conn: DbConn, sender: State<JobSender>) -> Json<String> {
    Json(String::from("acc"))
}

// https://github.com/SergioBenitez/Rocket/issues/714
use rocket::State;
use std::ops::Deref;
struct JobSender(SyncSender<ProducerData>);
impl Deref for JobSender {
    type Target = mpsc::SyncSender<ProducerData>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

embed_migrations!("./migrations");
#[allow(unused_imports)]
mod migrations {
    embed_migrations!();
    pub fn run_migrations(p: crate::db::Pool) {
        use std::io::stdout;
        embedded_migrations::run_with_output(&*p.get().expect("connection instance"), &mut stdout()).expect("Can't run migrations");
    }
}

fn launch_rocket(tx: &SyncSender<ProducerData>, p: &Pool) {
    migrations::run_migrations(p.clone());
    let config = Config::build(Environment::Development)
        .address("127.0.0.1")
        .port(8072)
        .workers(8)
        .log_level(LoggingLevel::Normal)
        .unwrap();

    let rocket = rocket::custom(config);
    let rocket = rocket.mount("/v1", routes![cac, cmt]);
    log::set_max_level(log::LevelFilter::max());
    let rocket = rocket.manage(p.clone()).manage(JobSender(tx.clone()));
    error!("Launch error {:#?}", rocket.launch());
}

fn main() {
    setup_logger(None);
    let group_id = "account";
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || send_loop(&rx));

    dotenv().ok();
    let database_url = env::var("DATABASE_URL_ACCOUNT").expect("DATABASE_URL_ACCOUNT must be set");
    let pool = db::init_pool(&database_url);

    let cac_handle = consume(
        group_id,
        "confirm_account_creation",
        Box::from(CacContext {
            sender: tx.clone(),
            pool: pool.clone()
        })
    );
    let cmt_handle = consume(
        group_id,
        "confirm_money_transfer",
        Box::from(CmtContext {
            sender: tx.clone(),
            pool: pool.clone()
        })
    );

    let api_handle = thread::spawn(move || launch_rocket(&tx, &pool.clone()));

    cac_handle.join().expect_err("Error closing cac handler");
    cmt_handle.join().expect_err("Error closing cmt handler");
    api_handle.join().expect_err("Error closing api handler");
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
