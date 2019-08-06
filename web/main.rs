#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
mod checks;
mod serverwatch_state;
mod api;
mod pages;

use rocket_contrib::templates;
use rocket_contrib::serve::{self, StaticFiles};

fn main() {
  rocket::ignite()
    .manage(serverwatch_state::init())
    .attach(templates::Template::fairing())
    .mount("/", pages::pages_routes())
    .mount("/api", api::api_routes())
    .mount("/assets", StaticFiles::new("assets/dist/", serve::Options::DotFiles))
    .launch();
}
