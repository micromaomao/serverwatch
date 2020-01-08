#![feature(proc_macro_hygiene, decl_macro, type_ascription)]

#[macro_use] extern crate rocket;
mod checks;
mod serverwatch_state;
mod api;
mod pages;
mod datastores;
mod push;

use rocket_contrib::templates;
use rocket_contrib::serve::{self, StaticFiles};

use std::fs;

fn main() {
  if !fs::metadata("Rocket.toml").expect("could not find Rocket.toml in cwd. Make sure cargo run is run with cwd = serverwatch/web.").is_file() {
    panic!("Rocket.toml is not a file.");
  }
  rocket::ignite()
    .manage(serverwatch_state::init())
    .attach(templates::Template::fairing())
    .mount("/", pages::pages_routes())
    .mount("/api", api::api_routes())
    .mount("/assets", StaticFiles::new("assets/dist/", serve::Options::DotFiles))
    .launch();
}
