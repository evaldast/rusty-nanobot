use node::{Account, Key};
use rusqlite::{Connection, Error};
use std::sync::Mutex;

// fn init_database(conn: &Connection) {
//     conn.execute(
//         "CREATE TABLE accounts (
//                   id                INTEGER PRIMARY KEY,
//                   account           TEXT NOT NULL,
//                   public            TEXT NOT NULL,
//                   private           TEXT NOT NULL,
//                   wallet            TEXT NOT NULL,
//                   email             TEXT UNIQUE NOT NULL
//                   )",
//         &[],
//     ).expect("create accounts table");
// }

pub fn add_account_hangouts(
    db_conn: &Mutex<Connection>,
    key: &Key,
    email: &str,
    wallet: &str,
) -> Result<i32, Error> {
    db_conn.lock()
        .expect("db connection lock")
        .execute("INSERT INTO accounts (account, public, private, wallet, email) VALUES (?1, ?2, ?3, ?4, ?5)", &[&key.account, &key.public, &key.private, &wallet, &email])
}

pub fn add_account_teams(
    db_conn: &Mutex<Connection>,
    key: &Key,
    teams_id: &str,
    wallet: &str,
) -> Result<i32, Error> {
    db_conn.lock()
        .expect("db connection lock")
        .execute("INSERT INTO accounts (account, public, private, wallet, teamsId) VALUES (?1, ?2, ?3, ?4, ?5)", &[&key.account, &key.public, &key.private, &wallet, &teams_id])
}

pub fn get_account_hangouts(db_conn: &Mutex<Connection>, email: &str) -> Result<Account, Error> {
    db_conn.lock().expect("db connection lock").query_row(
        "SELECT account, public, private, wallet, email FROM accounts WHERE email = ?",
        &[&email],
        |row| Account {
            account: row.get(0),
            public: row.get(1),
            private: row.get(2),
            wallet: row.get(3),
            email: row.get(4),
            teams_id: String::default(),
        },
    )
}

pub fn get_account_teams(db_conn: &Mutex<Connection>, teams_id: &str) -> Result<Account, Error> {
    db_conn.lock().expect("db connection lock").query_row(
        "SELECT account, public, private, wallet, teamsId FROM accounts WHERE teamsId = ?",
        &[&teams_id],
        |row| Account {
            account: row.get(0),
            public: row.get(1),
            private: row.get(2),
            wallet: row.get(3),
            teams_id: row.get(4),
            email: String::default(),
        },
    )
}

pub fn get_connection() -> Mutex<Connection> {
    let conn = Connection::open("sqlite/main.database").unwrap();

    Mutex::new(conn)
}
