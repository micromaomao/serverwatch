//! Provides some basic checker to either be called directly, or be passed to the
//! status page executor. The [`Checker`](crate::checkers::Checker) trait can be used
//! to implement your own checker to work with the status page.

use std::fmt::{Display, Formatter};
use std::fmt;

/// A `Checker` performs some server check, for example by making an http request
/// and expecting 200. The checker may also time the request and return with an
/// [`UNSTABLE`](crate::checkers::CheckResultType::UNSTABLE) result if the server
/// took too long to response, etc.
pub trait Checker {
	/// Performs the check.
	fn check(&mut self) -> CheckResult;
}

/// The result of a check, along with some additional information, if available.
#[derive(Debug)]
pub struct CheckResult {
  pub result_type: CheckResultType,
  /// optional information, which may be displayed by the [`check()`](crate::checkers::Checker::check) caller.
  pub info: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CheckResultType {
  /// <span style="color: green">Everything's good.</span>
  UP,
  /// <span style="color: darkorange">Something's not right.</span>
  UNSTABLE,
  /// <span style="color: red">Everything is wrong.</span>
  ERROR
}

impl CheckResult {
  /// Construct an `UP` result.
  pub fn up(info: Option<String>) -> Self {
    CheckResult{result_type: CheckResultType::UP, info}
  }
  /// Construct an `ERROR` result.
  pub fn error(info: Option<String>) -> Self {
    CheckResult{result_type: CheckResultType::ERROR, info}
  }
  /// Construct an `UNSTABLE` result.
  pub fn unstable(info: Option<String>) -> Self {
    CheckResult{result_type: CheckResultType::UNSTABLE, info}
  }

  pub fn expect(&self) {
    if self.result_type != CheckResultType::UP {
      panic!("expect on {:?} failed.", self);
    }
  }
  pub fn expect_err(&self) {
    if self.result_type != CheckResultType::ERROR {
      panic!("expect_err on {:?} failed.", self);
    }
  }
  pub fn expect_err_contains(&self, pattern: &str) {
    self.expect_err();
    match self.info {
      None => panic!("info is None."),
      Some(ref s) => match s.find(pattern) {
        None => panic!("pattern not found."),
        Some(_) => {}
      }
    }
  }
}

impl Display for CheckResult {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    if let Some(ref info) = self.info {
      write!(f, "{:?}: {}", self.result_type, info)
    } else {
      write!(f, "{:?}", self.result_type)
    }
  }
}

#[cfg(feature = "checkers")] pub mod http;
#[cfg(feature = "checkers")] pub mod tls;
