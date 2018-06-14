#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate erased_serde;

extern crate chrono;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate regex;
extern crate rocket;
extern crate rocket_contrib;
extern crate rusqlite;
extern crate serde;
extern crate tokio_core;

mod api;
mod db;
mod node;

fn main() {
    api::controller::rocket(db::get_connection()).launch();
}
