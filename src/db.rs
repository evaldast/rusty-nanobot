use std::sync::Mutex;
use rusqlite::{Connection, Error};
use rocket::State;

fn init_database(conn: &Connection) {
    conn.execute("CREATE TABLE entries (
                  id              INTEGER PRIMARY KEY,
                  name            TEXT NOT NULL
                  )", &[])
        .expect("create entries table");

    conn.execute("INSERT INTO entries (id, name) VALUES ($1, $2)",
            &[&0, &"Rocketeer"])
        .expect("insert single entry into entries table");
}

pub fn test(db_conn: State<Mutex<Connection>>) -> Result<String, Error>  {
    db_conn.lock()
        .expect("db connection lock")
        .query_row("SELECT name FROM entries WHERE id = 0",
                   &[], |row| { row.get(0) })
}

pub fn get_connection() -> Mutex<Connection> {
    let conn = Connection::open_in_memory().expect("in memory db");
    init_database(&conn);

    return Mutex::new(conn);
}
