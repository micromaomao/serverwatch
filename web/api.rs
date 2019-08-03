use rocket_contrib::json::Json;
use rocket::State;
use super::serverwatch_state::SwState;
use serde::Serialize;

#[derive(Serialize)]
pub struct StatusLogResponse {
	pub checks: Vec<CheckLogResponse>,
}

#[derive(Serialize)]
pub struct CheckLogResponse {
	pub desc: &'static str,
	pub log: Vec<CheckLogEntry>,
	pub last_state: &'static str,
}

#[derive(Serialize)]
pub struct CheckLogEntry {
	pub state: &'static str,
	pub info: String,
	pub time: u64,
	pub secs_ago: u32,
}

use serverwatch::checkers::CheckResultType;

pub fn result_type_to_string(r: CheckResultType) -> &'static str {
	match r {
		CheckResultType::UP => "up",
		CheckResultType::WARN => "warn",
		CheckResultType::ERROR => "error",
	}
}

pub fn get_status_log_response_struct(sw_state: State<SwState>) -> StatusLogResponse {
	let last_states: Vec<&'static str> = sw_state.schd.get_latest_results().iter().map(|s| {
		if let Some(s) = s {
			result_type_to_string(s.result_type)
		} else {
			"null"
		}
	}).collect();
	StatusLogResponse{
		checks: (*sw_state.result_store.read().unwrap()).iter().enumerate().map(|(i, res_log)| {
			CheckLogResponse{
				desc: sw_state.descs_list[i],
				log: res_log.iter().map(|(check_res, time)| {
					CheckLogEntry{
						state: result_type_to_string(check_res.result_type),
						info: match &check_res.info {
							Some(i) => i.clone(),
							None => "-".to_owned()
						},
						time: time.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
						secs_ago: time.elapsed().unwrap().as_secs() as u32,
					}
				}).collect(),
				last_state: last_states[i],
			}
		}).collect()
	}
}

#[get("/status_log")]
fn status_log(sw_state: State<SwState>) -> Json<StatusLogResponse> {
	Json(get_status_log_response_struct(sw_state))
}

pub fn api_routes() -> impl Into<Vec<rocket::Route>> {
	routes![status_log]
}
