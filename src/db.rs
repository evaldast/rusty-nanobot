use rocket::State;
use rusqlite::{Connection, Error};
use std::sync::Mutex;

fn init_database(conn: &Connection) {
    conn.execute(
        "CREATE TABLE accounts (
                  id                INTEGER PRIMARY KEY,
                  account           TEXT NOT NULL,
                  public            TEXT NOT NULL,
                  private           TEXT NOT NULL,
                  email             TEXT UNIQUE NOT NULL
                  )",
        &[],
    ).expect("create accounts table");
}

pub fn test(db_conn: State<Mutex<Connection>>) -> Result<String, Error> {
    db_conn.lock().expect("db connection lock").query_row(
        "SELECT * FROM accounts",
        &[],
        |row| row.get(0),
    )
}

pub fn get_connection() -> Mutex<Connection> {
    //let conn = Connection::open_in_memory().expect("in memory db");
    let conn = Connection::open("sqlite\\main.database").unwrap();
    // init_database(&conn);

    return Mutex::new(conn);
}
