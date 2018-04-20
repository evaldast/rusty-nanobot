use rusqlite::{Connection, Error};
use std::sync::Mutex;
use node::Account;

// fn init_database(conn: &Connection) {
//     conn.execute(
//         "CREATE TABLE accounts (
//                   id                INTEGER PRIMARY KEY,
//                   account           TEXT NOT NULL,
//                   public            TEXT NOT NULL,
//                   private           TEXT NOT NULL,
//                   email             TEXT UNIQUE NOT NULL
//                   )",
//         &[],
//     ).expect("create accounts table");
// }

pub fn add_account(db_conn: &Mutex<Connection>, acc: Account) -> Result<(), Error> {
    db_conn.lock()
        .expect("db connection lock")
        .execute("INSERT INTO accounts (account, public, private, email) VALUES (?1, ?2, ?3, ?4)", &[&acc.account, &acc.public, &acc.private, &acc.email])
        .expect("");

    return Ok(());
}

pub fn get_account(db_conn: &Mutex<Connection>, email: String) -> Result<Account, Error> {
    db_conn.lock()
        .expect("db connection lock")
        .query_row("SELECT account, public, private, email FROM accounts WHERE email = ?", &[&email], 
            |row| { Account { account: row.get(0), public: row.get(1), private: row.get(2), email: row.get(3) } })    
} 

pub fn get_connection() -> Mutex<Connection> {
    let conn = Connection::open("sqlite/main.database").unwrap();

    return Mutex::new(conn);
}
