use rocket::response::Content;
use rocket::http::ContentType;

#[get("/style.css")]
fn style_css() -> Content<&'static str> {
  Content(ContentType::CSS, include_str!(concat!(env!("OUT_DIR"), "/style.css")))
}

#[get("/script.js")]
fn script_js() -> Content<&'static str> {
  Content(ContentType::JavaScript, include_str!("assets/script.js"))
}

use super::api::get_status_log_response_struct;
use rocket_contrib::templates::Template;
use rocket::State;
use super::serverwatch_state::SwState;

#[get("/")]
fn index(sw_state: State<SwState>) -> Template {
  Template::render("index", get_status_log_response_struct(sw_state))
}

pub fn pages_routes() -> impl Into<Vec<rocket::Route>> {
  routes![style_css, script_js, index]
}
