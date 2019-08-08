use std::time;
use serverwatch::checkers::{CheckResult, CheckResultType};

pub type CheckId = u32;

pub type CheckLogId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckLog {
  pub time: time::SystemTime,
  pub result: CheckResult,
}

use std::error::Error;

#[derive(Clone, Debug)]
pub enum DatabaseErrorInnerStr {
  Owned(String),
  Static(&'static str),
}

#[derive(Debug)]
pub struct DatabaseError {
  pub inner_str: DatabaseErrorInnerStr,
  pub inner: Option<Box<dyn Error + 'static>>,
}
pub type DataResult<T> = Result<T, DatabaseError>;

use std::fmt;
impl fmt::Display for DatabaseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.inner_str {
      DatabaseErrorInnerStr::Owned(ref s) => write!(f, "{}", s),
      DatabaseErrorInnerStr::Static(s) => write!(f, "{}", s),
    }
  }
}

impl Error for DatabaseError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    self.inner.as_ref().map(|b| b.as_ref())
  }
}

impl DatabaseError {
  pub fn from_static_str(s: &'static str) -> Self {
    Self{inner: None, inner_str: DatabaseErrorInnerStr::Static(s)}
  }

  pub fn from_string(s: String) -> Self {
    Self{inner: None, inner_str: DatabaseErrorInnerStr::Owned(s)}
  }

  pub fn from_inner<T: Error + 'static>(inner: T) -> Self {
    Self{inner_str: DatabaseErrorInnerStr::Owned(format!("{}", &inner)), inner: Some(Box::new(inner))}
  }

  pub fn from_inner_and_str<T: Error + 'static>(inner: T, s: &'static str) -> Self {
    Self{inner: Some(Box::new(inner)), inner_str: DatabaseErrorInnerStr::Static(s)}
  }
}

use serde::Serialize;
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct LogCounts {
  pub num_up: u64,
  pub num_warn: u64,
  pub num_error: u64,
}

impl std::ops::Add for LogCounts {
  type Output = Self;
  fn add(self, rhs: Self) -> Self {
    Self{num_up: self.num_up + rhs.num_up, num_warn: self.num_warn + rhs.num_warn, num_error: self.num_error + rhs.num_error}
  }
}

impl std::ops::AddAssign for LogCounts {
  fn add_assign(&mut self, rhs: Self) {
    self.num_up += rhs.num_up;
    self.num_warn += rhs.num_warn;
    self.num_error += rhs.num_error;
  }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LogFilter {
  pub include_up: bool,
  pub include_warn: bool,
  pub include_error: bool,
  pub min_time: Option<time::SystemTime>,
  pub max_time: Option<time::SystemTime>,
}
impl Default for LogFilter {
  fn default() -> Self {
    Self{
      include_up: true,
      include_warn: true,
      include_error: true,
      min_time: None,
      max_time: None
    }
  }
}

impl LogFilter {
  pub fn after(t: time::SystemTime) -> Self {
    Self{
      include_up: true,
      include_warn: true,
      include_error: true,
      min_time: Some(t),
      max_time: None,
    }
  }
}

#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub enum LogOrder {
  Unordered,
  TimeAsc,
  TimeDesc,
}

pub trait DataStore: Send + Sync {
  fn add_log(&self, check_id: CheckId, log: CheckLog) -> DataResult<CheckLogId>;
  fn query_log(&self, id: CheckLogId) -> DataResult<CheckLog>;
  fn search_log<'a>(&'a self, check: CheckId, search: LogFilter, order: LogOrder, each_fn: Box<dyn FnMut(CheckLogId, CheckLog) -> bool + 'a>) -> DataResult<()>;
  fn count_logs(&self, check: CheckId, filter: LogFilter) -> DataResult<LogCounts>;
}

pub fn result_type_to_str(t: CheckResultType) -> &'static str {
  match t {
    CheckResultType::UP => "up",
    CheckResultType::WARN => "warn",
    CheckResultType::ERROR => "error",
  }
}

pub fn str_to_result_type(s: &str) -> Option<CheckResultType> {
  match s {
    "up" => Some(CheckResultType::UP),
    "warn" => Some(CheckResultType::WARN),
    "error" => Some(CheckResultType::ERROR),
    _ => None
  }
}

pub mod sqlite;
#[cfg(test)] mod test;
