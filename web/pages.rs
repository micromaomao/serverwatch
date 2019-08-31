use super::api::{get_status_log_response_struct, CheckLogResponse};
use rocket_contrib::templates::Template;
use rocket::State;
use rocket::response::Content;
use rocket::http::ContentType;
use super::serverwatch_state::SwState;
use serde::Serialize;

#[derive(Serialize)]
struct Ctx {
  pub check_state_json: String,
  pub checks: Vec<CheckLogResponse>,
}

#[get("/")]
fn index(sw_state: State<SwState>) -> Result<Template, String> {
  let check_state = get_status_log_response_struct(sw_state).map_err(|e| format!("Unable to fetch status from database: {}", &e))?;
  Ok(Template::render("index", Ctx{check_state_json: serde_json::to_string(&check_state).map_err(|e| format!("{}", &e))?, checks: check_state.checks}))
}

#[get("/sw.js")]
fn sw_js() -> Result<Content<String>, String> {
  use std::fs;
  Ok(Content(ContentType::JavaScript, fs::read_to_string("assets/dist/sw.js").map_err(|e| format!("{}", &e))?))
}

#[get("/application_server_key.base64")]
fn app_srv_key_b64(sw_state: State<SwState>) -> Content<String> {
  Content(ContentType::Plain, sw_state.app_server_pub_key_b64.to_owned())
}

pub fn pages_routes() -> impl Into<Vec<rocket::Route>> {
  routes![index, sw_js, app_srv_key_b64]
}
