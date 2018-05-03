#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate rocket;
extern crate rocket_contrib;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate rusqlite;
extern crate regex;

mod api;
mod db;
mod node;

fn main() {
    api::rocket(db::get_connection()).launch();
}
