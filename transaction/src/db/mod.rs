pub mod models;
pub mod schema;
pub mod util;

pub use self::models::*;

use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn connect(database_url: &String) -> Pool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder().build(manager).expect("Failed to create pool")
}
