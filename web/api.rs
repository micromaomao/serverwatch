use rocket_contrib::json::Json;
use rocket::State;
use super::serverwatch_state::SwState;
use serde::Serialize;
use super::datastores;
use datastores::DataStore;

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
}

use serverwatch::checkers::CheckResultType;

pub fn result_type_to_string(r: CheckResultType) -> &'static str {
	match r {
		CheckResultType::UP => "up",
		CheckResultType::WARN => "warn",
		CheckResultType::ERROR => "error",
	}
}

pub fn get_status_log_response_struct(sw_state: State<SwState>) -> Result<StatusLogResponse, datastores::DatabaseError> {
	let last_states: Vec<&'static str> = sw_state.schd.get_latest_results().iter().map(|s| {
		if let Some(s) = s {
			result_type_to_string(s.result_type)
		} else {
			"null"
		}
	}).collect();
	Ok(StatusLogResponse{
		checks: {
			let iter = sw_state.checkids_list.iter().enumerate().map(|(index, check_id)| {
				let desc = sw_state.descs_list[index];
				Ok(CheckLogResponse{
					desc,
					log: {
						let mut log = Vec::new();
						sw_state.data_store.search_log(*check_id, datastores::LogFilter::default(), datastores::LogOrder::TimeDesc, |_, log_entry| {
							log.push(CheckLogEntry{
								state: result_type_to_string(log_entry.result.result_type),
								info: match log_entry.result.info {
									Some(s) => s,
									None => "-".to_owned(),
								},
								time: log_entry.time.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
							});
							true
						})?;
						log
					},
					last_state: last_states[index]
				})
			});
			let mut checks = Vec::new();
			for r in iter {
				match r {
					Ok(k) => checks.push(k),
					Err(e) => return Err(e),
				}
			};
			checks
		}
	})
}

#[get("/status_log")]
fn status_log(sw_state: State<SwState>) -> Result<Json<StatusLogResponse>, String> {
	Ok(Json(get_status_log_response_struct(sw_state).map_err(|e| format!("Database error: {}", &e))?))
}

pub fn api_routes() -> impl Into<Vec<rocket::Route>> {
	routes![status_log]
}
