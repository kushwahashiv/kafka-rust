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
#[macro_use]
extern crate serde_json;

use dotenv::dotenv;
use std::env;

mod db;
mod kafka_consumer;
mod kafka_producer;
mod logger;

use crate::db::models::{Account, Transactions};

use crate::db::Pool;
use crate::kafka_consumer::{consume, ValuesProcessor};
use crate::kafka_producer::get_producer;
use crate::logger::setup_logger;
use avro_rs::types::Value;
use chrono::Utc;
use db::DbConn;
use diesel::pg::PgConnection;
use log::{error, info};
use rocket::handler::Outcome;
use rocket::http::Method::*;
use rocket::{Data, Request, Route};
use rocket_contrib::json::Json;
use schema_registry_converter::schema_registry::SubjectNameStrategy;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;

struct ProducerData {
    topic: &'static str,
    key: String,
    values: Vec<(&'static str, Value)>
}

struct AccContext {
    sender: SyncSender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for AccContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_acc(values, &self.pool.get().unwrap(), &self.sender)
    }
}

fn handle_acc(values: &[(String, Value)], conn: &PgConnection, sender: &SyncSender<ProducerData>) {
    let id = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an id, while that was expected")
    };
    let account_no = match &values[1] {
        (_account_no, Value::String(ref v)) => v.clone(),
        _ => panic!("Not account no, while that was expected")
    };
    let token = match &values[2] {
        (_token, Value::String(ref v)) => v.clone(),
        _ => panic!("Not account no, while that was expected")
    };
    let account_type = match &values[3] {
        (_account_type, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an account type, while that was expected")
    };
    // sender.send(producer_data).unwrap();
}

struct AcfContext {
    sender: SyncSender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for AcfContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_acf(values, &self.pool.get().unwrap(), &self.sender)
    }
}

fn handle_acf(values: &[(String, Value)], conn: &PgConnection, sender: &SyncSender<ProducerData>) {
    let id = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an id, while that was expected")
    };
    // sender.send(producer_data).unwrap();
}

struct MtcContext {
    sender: SyncSender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for MtcContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_mtc(values, &self.pool.get().unwrap(), &self.sender)
    }
}

fn handle_mtc(values: &[(String, Value)], conn: &PgConnection, sender: &SyncSender<ProducerData>) {
    let id = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an id, while that was expected")
    };
    // sender.send(producer_data).unwrap();
}

struct MtfContext {
    sender: SyncSender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for MtfContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_mtc(values, &self.pool.get().unwrap(), &self.sender)
    }
}

fn handle_mtf(values: &[(String, Value)], conn: &PgConnection, sender: &SyncSender<ProducerData>) {
    let id = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an id, while that was expected")
    };
    // sender.send(producer_data).unwrap();
}

struct BcContext {
    sender: SyncSender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for BcContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_bc(values, &self.pool.get().unwrap(), &self.sender)
    }
}

fn handle_bc(values: &[(String, Value)], conn: &PgConnection, sender: &SyncSender<ProducerData>) {
    let id = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an id, while that was expected")
    };
    // sender.send(producer_data).unwrap();
}

// https://github.com/SergioBenitez/Rocket/issues/714
use std::ops::Deref;
use rocket::State;
struct JobSender(SyncSender<ProducerData>);
impl Deref for JobSender {
    type Target = mpsc::SyncSender<ProducerData>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct LoginData {
    username: String,
    password: String
}

#[post("/login", data = "<data>")]
fn login(data: Json<LoginData>, conn: DbConn, sender: State<JobSender>) -> &'static str {
    let data: LoginData = data.into_inner();
    let account: Account = db::Account::get_account(data.username, data.password, &conn);
    let key = account.id.clone();

    let id = ("id", Value::String(key.clone()));
    let username = ("username", Value::String(account.username));
    let password = ("password", Value::String(account.password));

    let producer_data = ProducerData {
        topic: "confirm_account_creation",
        key,
        values: vec![id, username, password]
    };

    sender.try_send(producer_data).unwrap();
    "done"
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

fn main() {
    setup_logger(None);
    let group_id = "account";
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || send_loop(&rx));

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::init_pool(&database_url);
    migrations::run_migrations(pool.clone());

    rocket::ignite()
    .mount("/login", routes![login])
    .manage(pool.clone())
    .manage(JobSender(tx.clone()))
    .launch();

    log::set_max_level(log::LevelFilter::max());

    let acc_handle = consume(
        group_id,
        "account_creation_confirmed",
        Box::from(AccContext {
            sender: tx.clone(),
            pool: pool.clone()
        })
    );
    let acf_handle = consume(
        group_id,
        "account_creation_failed",
        Box::from(AcfContext {
            sender: tx.clone(),
            pool: pool.clone()
        })
    );
    let mtc_handle = consume(
        group_id,
        "money_transfer_confirmed",
        Box::from(MtcContext {
            sender: tx.clone(),
            pool: pool.clone()
        })
    );
    let mtf_handle = consume(
        group_id,
        "money_transfer_failed",
        Box::from(MtfContext {
            sender: tx.clone(),
            pool: pool.clone()
        })
    );
    let bc_handle = consume(
        group_id,
        "balance_changed",
        Box::from(BcContext {
            sender: tx.clone(),
            pool: pool.clone()
        })
    );
    acc_handle.join().expect_err("Error closing acc handler");
    acf_handle.join().expect_err("Error closing acf handler");
    mtc_handle.join().expect_err("Error closing mtc handler");
    mtf_handle.join().expect_err("Error closing mtf handler");
    bc_handle.join().expect_err("Error closing bc handler");
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
