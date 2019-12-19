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

// use crate::db::models::{Acc, ConfirmedAccount};

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
    let group_id = "account";
    // let (sender, receiver) = mpsc::channel();
    // thread::spawn(move || send_loop(&receiver));

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = db::connect(&database_url);
    migrations::run_migrations(pool.clone());
    /*launch_rocket(pool.clone());

    let cac_handle = consume(
        group_id,
        "confirm_account_creation",
        Box::from(AccountContext {
            sender: sender.clone(),
            pool: pool.clone()
        })
    );
    let cmt_handle = consume(
        group_id,
        "confirm_money_transfer",
        Box::from(BalanceContext {
            sender: sender.clone(),
            pool: pool.clone()
        })
    );
    cac_handle.join().expect_err("Error closing cac handler");
    cmt_handle.join().expect_err("Error closing cmt handler");*/
}

struct ProducerData {
    topic: &'static str,
    key: String,
    values: Vec<(&'static str, Value)>
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
