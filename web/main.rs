#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
mod checks;
mod serverwatch_state;
mod api;
mod pages;

use rocket_contrib::templates;

fn main() {
  rocket::ignite()
    .manage(serverwatch_state::init())
    .attach(templates::Template::fairing())
    .mount("/", pages::pages_routes())
    .mount("/api", api::api_routes())
    .launch();
}
