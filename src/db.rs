use rusqlite::{Connection, Error};
use std::sync::Mutex;
use node::NewAccount;

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

pub fn add_account(db_conn: &Mutex<Connection>, acc: NewAccount) -> Result<(), Error> {
    db_conn.lock()
        .expect("db connection lock")
        .execute("INSERT INTO accounts (account, public, private, email) VALUES (?1, ?2, ?3, ?4)", &[&acc.account, &acc.public, &acc.private, &acc.email])
        .expect("asd");

    return Ok(());
}

pub fn get_connection() -> Mutex<Connection> {
    let conn = Connection::open("sqlite\\main.database").unwrap();
    init_database(&conn);

    return Mutex::new(conn);
}
