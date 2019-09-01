use rocket_contrib::json::Json;
use rocket::State;
use super::serverwatch_state::SwState;
use serde::{Serialize, Deserialize};
use super::datastores;
use super::push;
use std::time;

#[derive(Serialize)]
pub struct StatusLogResponse {
	pub checks: Vec<CheckLogResponse>,
}

#[derive(Serialize)]
pub struct CheckLogResponse {
	pub id: datastores::CheckId,
	pub desc: &'static str,
	pub log: Vec<CheckLogEntry>,
	pub last_state: &'static str,
	pub statistics: QuickStatistics,
}

#[derive(Serialize)]
pub struct CheckLogEntry {
	pub id: datastores::CheckLogId,
	pub state: &'static str,
	pub info: String,
	pub time: u64,
}

#[derive(Serialize)]
pub struct QuickStatistics {
	pub last_day: datastores::LogCounts,
	pub last_7_day: datastores::LogCounts,
	pub last_month: datastores::LogCounts,
}

use serverwatch::checkers::CheckResultType;

use std::collections::HashMap;

#[derive(Deserialize)]
pub struct NotificationPost {
	pub noti_state: HashMap<datastores::CheckId, NotificationItem>,
	pub sub: push::PushSubscriptionJson
}

#[derive(Deserialize)]
pub struct NotificationItem {
	pub notify_warn: bool
}

#[derive(Deserialize)]
pub struct PushTaskData {
	pub sub: push::PushSubscriptionJson
}

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
	let now = time::SystemTime::now();
	use datastores::LogFilter;
	use time::Duration;
	let day = Duration::from_secs(24*60*60);
	Ok(StatusLogResponse{
		checks: {
			let iter = sw_state.checkids_list.iter().enumerate().map(|(index, check_id)| {
				let desc = sw_state.descs_list[index];
				Ok(CheckLogResponse{
					id: *check_id,
					desc,
					log: {
						let mut log = Vec::new();
						sw_state.data_store.search_log(*check_id, LogFilter::after(now - Duration::from_secs(10*60)), datastores::LogOrder::TimeDesc, Box::new(|log_id, log_entry| {
							log.push(CheckLogEntry{
								id: log_id,
								state: result_type_to_string(log_entry.result.result_type),
								info: match log_entry.result.info {
									Some(s) => s,
									None => "-".to_owned(),
								},
								time: log_entry.time.duration_since(time::UNIX_EPOCH).unwrap().as_millis() as u64,
							});
							true
						}))?;
						log
					},
					last_state: last_states[index],
					statistics: QuickStatistics{
						last_day: sw_state.data_store.count_logs(*check_id, LogFilter::after(now - day))?,
						last_7_day: sw_state.data_store.count_logs(*check_id, LogFilter::after(now - 7*day))?,
						last_month: sw_state.data_store.count_logs(*check_id, LogFilter::after(now - 30*day))?
					}
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

#[post("/notification", data = "<task>")]
fn set_notification(sw_state: State<SwState>, task: Json<NotificationPost>) -> Result<rocket::Response, rocket::Response> {
	macro_rules! report_error {
		($status_code:expr, $err_str:expr) => {
			return Err(rocket::Response::build().status(rocket::http::Status::raw($status_code)).sized_body(std::io::Cursor::new($err_str.to_owned())).finalize());
		};
	}
	let (endpoint_url, auth, p256dh) = match push::decode_sub_json(&task.sub) {
		Ok(k) => k,
		Err(e) => report_error!(400, e)
	};

	let mut sub_list = Vec::new();
	sub_list.reserve_exact(task.noti_state.len());
	for (check_id, nt_item) in task.noti_state.iter() {
		sub_list.push(datastores::PushSubscription{
			check_id: *check_id,
			notify_warn: nt_item.notify_warn
		});
	}
	if let Err(e) = sw_state.data_store.update_push_subscriptions(endpoint_url.as_str(), &auth, &p256dh, &sub_list) {
		return Err(rocket::Response::build().status(rocket::http::Status::raw(500)).sized_body(std::io::Cursor::new(format!("{}", e))).finalize());
	}
	Ok(rocket::Response::build().raw_status(200, "Done").finalize())
}

#[post("/notification/test", data = "<push_test_data>")]
fn push_test(sw_state: State<SwState>, push_test_data: Json<PushTaskData>) -> rocket::Response {
	macro_rules! report_error {
		($status_code:expr, $err_str:expr) => {
			return rocket::Response::build().status(rocket::http::Status::raw($status_code)).sized_body(std::io::Cursor::new($err_str.to_owned())).finalize();
		};
	}
	let (endpoint_url, auth, p256dh) = match push::decode_sub_json(&push_test_data.sub) {
		Ok(k) => k,
		Err(e) => report_error!(400, e)
	};
	sw_state.push_queue.lock().unwrap().send((endpoint_url, p256dh, auth, format!("push_test\n{}\nThis is what your notification will look like.\nNotifications will deliver even when this page is closed.", time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_millis()), time::Duration::from_secs(24*60*60), "push_test".to_owned()));
	return rocket::Response::build().status(rocket::http::Status::raw(200)).finalize();
}

pub fn api_routes() -> impl Into<Vec<rocket::Route>> {
	routes![status_log, set_notification, push_test]
}
