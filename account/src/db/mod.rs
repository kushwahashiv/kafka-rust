pub mod models;
pub mod schema;
pub mod util;

pub use self::models::*;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use r2d2;
use rocket::{http::Status,
             request::{self, FromRequest},
             Outcome,
             Request,
             State};
use std::ops::Deref;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

type Connection = diesel::pg::PgConnection;
pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<Connection>>);

pub fn init_pool(database_url: &String) -> Pool {
    let manager = ConnectionManager::new(database_url);
    r2d2::Pool::builder().build(manager).expect("Failed to create pool")
}

impl<'a, 'r> FromRequest<'a, 'r> for DbConn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<DbConn, ()> {
        let pool = request.guard::<State<Pool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ()))
        }
    }
}

impl Deref for DbConn {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
