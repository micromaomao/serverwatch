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
	pub state_upper_case: &'static str,
	pub state_material_icon: &'static str,
	pub info: String,
	pub time: u64,
	pub cssid: u32,
}

use serverwatch::checkers::CheckResultType;

pub fn result_type_to_string(r: CheckResultType) -> &'static str {
	match r {
		CheckResultType::UP => "up",
		CheckResultType::WARN => "warn",
		CheckResultType::ERROR => "error",
	}
}

pub fn result_type_to_string_upper(r: CheckResultType) -> &'static str {
	match r {
		CheckResultType::UP => "UP",
		CheckResultType::WARN => "WARN",
		CheckResultType::ERROR => "ERROR",
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
	let mut cssid = 0u32;
	StatusLogResponse{
		checks: (*sw_state.result_store.read().unwrap()).iter().enumerate().map(|(i, res_log)| {
			CheckLogResponse{
				desc: sw_state.descs_list[i],
				log: res_log.iter().map(|(check_res, time)| {
					cssid += 1;
					CheckLogEntry{
						state: result_type_to_string(check_res.result_type),
						state_upper_case: result_type_to_string_upper(check_res.result_type),
						state_material_icon: match check_res.result_type {
							CheckResultType::UP => "done",
							CheckResultType::WARN => "error",
							CheckResultType::ERROR => "clear",
						},
						info: match &check_res.info {
							Some(i) => i.clone(),
							None => "-".to_owned()
						},
						time: time.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
						cssid,
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
