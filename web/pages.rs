use super::api::{get_status_log_response_struct, CheckLogResponse};
use rocket_contrib::templates::Template;
use rocket::State;
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

pub fn pages_routes() -> impl Into<Vec<rocket::Route>> {
  routes![index]
}
