//! Simple http checks.

use crate::checkers::{Checker, CheckResult, CheckResultType};
use crate::utils::with_timeout;
use reqwest;
use std::ops::Deref;
use std::fmt::Write;
use std::time;
use std::sync::{RwLock, RwLockReadGuard};

lazy_static!{
	static ref SHARED_CLIENT: RwLock<reqwest::Client> = RwLock::new(new_http_client());
}

fn new_http_client() -> reqwest::Client {
	reqwest::ClientBuilder::new().redirect(reqwest::RedirectPolicy::none()).timeout(None).build().unwrap()
}
fn acquire_client<'b>() -> ClientGuard<'b> {
	match SHARED_CLIENT.read() {
		Ok(lg) => ClientGuard::LOCKED(lg),
		Err(_) => ClientGuard::DIRECT(new_http_client())
	}
}

enum ClientGuard<'a> {
	LOCKED(RwLockReadGuard<'a, reqwest::Client>),
	DIRECT(reqwest::Client)
}

impl<'a> Deref for ClientGuard<'a> {
	type Target = reqwest::Client;

	fn deref(&self) -> &Self::Target {
		match self {
			ClientGuard::LOCKED(lockguard) => lockguard.deref(),
			ClientGuard::DIRECT(client) => client
		}
	}
}

pub type ExpectFn<'a> = Box<dyn (Fn(&mut reqwest::Response) -> CheckResult) + Send + Sync + 'a>;

/// Performs a http check. Redirects are not followed.
///
/// ## Example
///
/// ```rust
/// use serverwatch::checkers::{Checker, http::HttpChecker};
/// let mut checker = HttpChecker::new("https://www.google.com/").unwrap();
/// checker.expect_200();
/// checker.check().expect();
/// ```
pub struct HttpChecker<'a> {
	url: reqwest::Url,
	expects: Vec<ExpectFn<'a>>,
	warn_timeout: time::Duration,
	err_timeout: time::Duration,
}

impl<'a> HttpChecker<'a> {
	pub fn new(url: &str) -> Result<Self, reqwest::UrlError> {
		acquire_client();
		let parsed_url = reqwest::Url::parse(url)?;
		Ok(HttpChecker{url: parsed_url, expects: Vec::new(), warn_timeout: time::Duration::from_secs(30), err_timeout: time::Duration::from_secs(30)})
	}

	/// Make sure the server response specifies certain condition&hellip;
	///
	/// ## Example
	/// ```rust
	/// use serverwatch::checkers::{Checker, http::HttpChecker, CheckResult};
	/// let mut checker = HttpChecker::new("https://google.com/generate_204").unwrap();
	/// checker.expect(Box::new(|res| {
	///   let t = match res.text() {
	///     Ok(t) => t,
	///     Err(e) => {return CheckResult::error(Some(format!("Could not convert response to text: {:?}", &e)))}
	///   };
	///   if t.len() != 0 {
	///     return CheckResult::error(Some(format!("Expected an empty response. Got {:?}", &t)));
	///   }
	///   CheckResult::up(None)
	/// }));
	/// checker.check().expect();
	/// ```
	pub fn expect(&mut self, func: ExpectFn<'a>) -> &mut Self {
		self.expects.push(func);
		self
	}

	pub fn expect_200(&mut self) -> &mut Self {
		self.expect_status(200)
	}

	pub fn expect_status(&mut self, status: u16) -> &mut Self {
		self.expect(Box::new(move |res| {
			if res.status().as_u16() != status {
				CheckResult::error(Some(format!("Expected status to be {}, got {}.", status, res.status())))
			} else {
				CheckResult::up(None)
			}
		}))
	}

	/// Add a test so that if the response does not contains the string `find`, check returns `ERROR`.
	pub fn expect_response_contains(&mut self, find: &'a str) -> &mut Self {
		self.expect(Box::new(move |res| {
			let text = match res.text() {
				Ok(t) => t,
				Err(e) => { return CheckResult::error(Some(format!("unable to parse response body as text: {}", &e))) }
			};
			if text.find(find).is_none() {
				CheckResult::error(Some(format!("{} not found in response body.", find)))
			} else {
				CheckResult::up(None)
			}
		}))
	}

	/// Set a time limit for the request.
	///
	/// * If the response arrives within `warn`, check result is `UP`.
	/// * If the response arrives after `warn` but before `error`, check result is `WARN` with the
	///   message including something like `server took ??ms to response.`.
	/// * Otherwise, result is `ERROR`.
	///
	/// ## Panics
	///
	/// Panics if `warn` is longer than `error`.
	pub fn set_timeouts(&mut self, warn: time::Duration, error: time::Duration) -> &mut Self {
		if warn > error {
			panic!("warn > error");
		}
		self.warn_timeout = warn;
		self.err_timeout = error;
		self
	}
}

#[test]
fn expect_status() {
	let five_secs = time::Duration::from_secs(5);
	macro_rules! check {
		($url:expr,$status:expr) => {{
			let mut checker = HttpChecker::new($url).unwrap();
			checker.expect_status($status);
			checker.set_timeouts(five_secs, five_secs);
			checker.check().expect();
		}};
	}
	macro_rules! check_fail {
		($url:expr,$status:expr) => {{
			let mut checker = HttpChecker::new($url).unwrap();
			checker.expect_status($status);
			checker.set_timeouts(five_secs, five_secs);
			checker.check().expect_err();
		}};
	}
	check!("https://www.google.com/", 200);
	check!("https://google.com/generate_204", 204);
	check!("https://google.com/generate_404", 404);
	check!("https://google.com/", 301);
	check_fail!("https://google.com/", 200);
	check_fail!("https://www.google.com/", 404);
	check_fail!("https://google.com/generate_204", 200);
	check_fail!("https://google.com/generate_404", 200);
	check_fail!("https://google.com/generate_404", 204);
}

#[test]
fn expect_response_contains() {
	let mut checker = HttpChecker::new("https://github.com").unwrap();
	checker.expect_200().expect_response_contains("GitHub is where people build software.");
	checker.check().expect();
	checker.expect_response_contains("GitHub is evil");
	checker.check().expect_err();
}

#[test]
fn should_timeout() {
	let mut checker = HttpChecker::new("https://www.google.com").unwrap();
	checker.set_timeouts(time::Duration::from_millis(1), time::Duration::from_millis(2));
	let measure = time::Instant::now();
	checker.check().expect_err();
	assert!(measure.elapsed() < time::Duration::from_millis(100));
	let mut checker = HttpChecker::new("https://www.google.com").unwrap();
	checker.set_timeouts(time::Duration::from_millis(1), time::Duration::from_millis(5000));
	let check_res = checker.check();
	assert_eq!(check_res.result_type, CheckResultType::WARN);
	if check_res.info.as_ref().unwrap().find("Server took").is_none() {
		panic!("Info returned is {:?}, which does not include {:?}.", check_res.info, "Server took");
	}
}


#[test]
fn fail_when_error() {
	let mut checker = HttpChecker::new("https://localhost").unwrap();
	checker.check().expect_err();
}

impl<'a> Checker for HttpChecker<'a> {
	fn check(&mut self) -> CheckResult {
		let url = self.url.clone();
		let start = time::Instant::now();
		if let Some(res) = with_timeout(move || acquire_client().get(url).send(), self.err_timeout) {
			let time_used = start.elapsed();
			match res {
				Ok(mut response) => {
					let mut warn_results = Vec::new();
					if time_used > self.warn_timeout {
						warn_results.push(CheckResult::warn(Some(format!("Server took {}ms to response.", time_used.as_millis()))));
					}
					let mut infos = Vec::new();
					for check_fn in self.expects.iter() {
						let check_res = (*check_fn)(&mut response);
						match check_res.result_type {
							CheckResultType::ERROR => { return check_res },
							CheckResultType::WARN => { warn_results.push(check_res) },
							CheckResultType::UP => {
								if let Some(info) = check_res.info {
									infos.push(info);
								}
							}
						}
					}
					if warn_results.len() > 0 {
						let mut f = String::new();
						write!(f, "{} expect checks reported WARN: ", warn_results.len()).unwrap();
						let mut is_first = true;
						for usr in warn_results.iter() {
							if !is_first {
								write!(f, ", ").unwrap();
							}
							is_first = false;
							if let Some(ref info) = usr.info {
								write!(f, "{}", info).unwrap();
							} else {
								write!(f, "(no info)").unwrap();
							}
						}
						CheckResult::warn(Some(f))
					} else {
						if infos.len() > 0 {
							CheckResult::up(Some(infos.join("\n")))
						} else {
							CheckResult::up(None)
						}
					}
				},
				Err(err) => {
					CheckResult::error(Some(format!("Failed to send request: {}", &err)))
				}
			}
		} else {
			CheckResult::error(Some(format!("Timeout of {}ms reached while making the request.", self.err_timeout.as_millis())))
		}
	}
}
