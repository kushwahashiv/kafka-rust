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

use crate::db::models::Account;

use crate::db::Pool;
use crate::kafka_consumer::{consume, ValuesProcessor};
use crate::kafka_producer::get_producer;
use crate::logger::setup_logger;
use avro_rs::types::Value;
use db::DbConn;
use diesel::pg::PgConnection;
use log::{error, info};
use rocket::config::{Config, Environment, LoggingLevel};
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
        handle_acc(values, &DbConn(self.pool.get().unwrap()), &self.sender)
    }
}

fn handle_acc(values: &[(String, Value)], conn: &PgConnection, sender: &SyncSender<ProducerData>) {
    let key = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an id, while that was expected")
    };

    let account_no = match &values[1] {
        (_account_no, Value::String(ref v)) => ("account_no", Value::String(v.clone())),
        _ => panic!("Not an account_no, while that was expected")
    };

    let token = match &values[2] {
        (_token, Value::String(ref v)) => ("token", Value::String(v.clone())),
        _ => panic!("Not token, while that was expected")
    };

    let reason = match &values[3] {
        (_reason, Value::String(ref v)) => ("reason", Value::String(v.clone())),
        _ => panic!("Not reason, while that was expected")
    };

    /*
    let producer_data = ProducerData {
        topic: "confirm_money_transfer",
        key,
        values: vec![account_no, token, reason]
    };

    sender.send(producer_data).unwrap();
    */
}

struct AcfContext {
    sender: SyncSender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for AcfContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_acf(values, &DbConn(self.pool.get().unwrap()), &self.sender)
    }
}

fn handle_acf(values: &[(String, Value)], conn: &DbConn, sender: &SyncSender<ProducerData>) {
    let id = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an id, while that was expected")
    };

    let reason = match &values[1] {
        (_reason, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an reason, while that was expected")
    };

    if reason.is_empty() {
        db::Account::remove_account(id.clone(), conn);
    }
}

struct MtcContext {
    sender: SyncSender<ProducerData>,
    pool: Pool
}

impl ValuesProcessor for MtcContext {
    fn process(&mut self, values: &[(String, Value)]) {
        handle_mtc(values, &DbConn(self.pool.get().unwrap()), &self.sender)
    }
}

fn handle_mtc(values: &[(String, Value)], conn: &DbConn, sender: &SyncSender<ProducerData>) {
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
        handle_mtf(values, &DbConn(self.pool.get().unwrap()), &self.sender)
    }
}

fn handle_mtf(values: &[(String, Value)], conn: &DbConn, sender: &SyncSender<ProducerData>) {
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
        handle_bc(values, &DbConn(self.pool.get().unwrap()), &self.sender)
    }
}

fn handle_bc(values: &[(String, Value)], conn: &DbConn, sender: &SyncSender<ProducerData>) {
    let id = match &values[0] {
        (_id, Value::String(ref v)) => v.clone(),
        _ => panic!("Not an id, while that was expected")
    };
    // sender.send(producer_data).unwrap();
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

#[derive(Deserialize, Serialize)]
#[allow(non_snake_case)]
struct LoginData {
    username: String,
    password: String
}

#[post("/login", data = "<data>")]
fn login(data: Json<LoginData>, conn: DbConn, sender: State<JobSender>) -> Json<Account> {
    let data: LoginData = data.into_inner();
    let acc: Account = db::Account::get_account(data.username, data.password, &conn);
    let key = acc.id.clone();

    let id = ("id", Value::String(key.clone()));
    let _type = ("_type", Value::String(String::from("MANUAL")));

    let producer_data = ProducerData {
        topic: "confirm_account_creation",
        key,
        values: vec![id, _type]
    };

    sender.try_send(producer_data).unwrap();
    Json(acc)
}

#[derive(Deserialize, Serialize)]
#[allow(non_snake_case)]
struct MoneyTransfer {
    id: String,
    token: String,
    amount: f64,
    from: String,
    to: String,
    description: String
}
#[post("/tx", data = "<data>")]
fn transact(data: Json<MoneyTransfer>, conn: DbConn, sender: State<JobSender>) -> Json<MoneyTransfer> {
    let data: MoneyTransfer = data.into_inner();
    let key = data.id.clone();

    let id = ("id", Value::String(key.clone()));
    let token = ("token", Value::String(data.token.clone()));
    let amount = ("amount", Value::Double(data.amount.clone()));
    let from = ("from", Value::String(data.from.clone()));
    let to = ("to", Value::String(data.to.clone()));
    let description = ("description", Value::String(data.description.clone()));

    let producer_data = ProducerData {
        topic: "confirm_money_transfer",
        key,
        values: vec![id, token, amount, from, to, description]
    };

    sender.try_send(producer_data).unwrap();
    Json(data)
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
        .port(8071)
        .workers(8)
        .log_level(LoggingLevel::Normal)
        .unwrap();
    let rocket = rocket::custom(config);
    let rocket = rocket.mount("/v1", routes![login, transact]);
    log::set_max_level(log::LevelFilter::max());
    let rocket = rocket.manage(p.clone()).manage(JobSender(tx.clone()));
    error!("Launch error {:#?}", rocket.launch());
}

fn main() {
    setup_logger(None);

    let group_id = "transaction";
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || send_loop(&rx));

    dotenv().ok();
    let database_url = env::var("DATABASE_URL_TRANSACTION").expect("DATABASE_URL_TRANSACTION must be set");
    let pool = db::init_pool(&database_url);

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

    let api_handle = thread::spawn(move || launch_rocket(&tx.clone(), &pool.clone()));

    acc_handle.join().expect_err("Error closing acc handler");
    acf_handle.join().expect_err("Error closing acf handler");
    mtc_handle.join().expect_err("Error closing mtc handler");
    mtf_handle.join().expect_err("Error closing mtf handler");
    bc_handle.join().expect_err("Error closing bc handler");
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
