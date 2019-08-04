//! The simple checker scheduler.
//!
//! Requires you not to change the list of checkers after initialization.
//!
//! Start by calling
//! [`SimpleSchd::new`](crate::scheduler::simple_schd::SimpleSchd::new).

use crate::checkers::{Checker, CheckResult, CheckResultType};
use std::time;
use std::collections::binary_heap::BinaryHeap;
use std::sync::{Mutex, Condvar, RwLock, RwLockReadGuard};

/// The simple checker scheduler.
pub struct SimpleSchd {
  queue: Mutex<BinaryHeap<NextCheck>>,
  checks: Vec<InnerCheck>,
  log: Mutex<Vec<LogEntry>>,
  log_wait: Condvar,
  latest_results: RwLock<Vec<Option<CheckResult>>>,
}

impl SimpleSchd {
  /// Initialize a new `SimpleSchd` with a list of checks. This list can't be
  /// changed or appended after initialization (this is a **simple** scheduler,
  /// after all).
  ///
  /// Checks are executed by calling
  /// [`self.step()`](crate::scheduler::simple_schd::SimpleSchd::step) in a loop,
  /// possibly on several threads.
  pub fn new(checks: Vec<Check>) -> Self {
    let now = time::Instant::now();
    use std::iter::FromIterator;
    let bh = BinaryHeap::from_iter(
      checks.iter().enumerate().map(|(i, _check)| {
        NextCheck{
          check_index: i,
          // Initially, all tasks are executed once without waiting for their delay.
          scheduled_time: now
        }
      })
    );
    let num_checks = checks.len();
    Self{
      queue: Mutex::new(bh),
      checks: checks.into_iter().map(|check| InnerCheck{
        checker: Mutex::new(check.checker),
        min_check_interval: check.min_check_interval,
        desc: check.desc,
      }).collect(),
      log: Mutex::new(Vec::new()),
      log_wait: Condvar::new(),
      latest_results: RwLock::new(vec![None; num_checks]),
    }
  }

  /// Do one check, updating internal states thread-safely.
  ///
  /// This function will wait for the appropriate amount of time if no checks is
  /// due now.
  ///
  /// This function is designed to be called from multiple threads. Spawn your
  /// desired number of checker threads and in it call this function in a loop.
  ///
  /// Return whether any check is executed. (Could be false if there are no more
  /// checks to run)
  pub fn step(&self) -> bool {
    let now = time::Instant::now();
    let nc = match self.queue.lock().unwrap().pop() {
      Some(k) => k,
      None => {
        return false;
      }
    };
    if nc.scheduled_time > now {
      std::thread::sleep(nc.scheduled_time - now);
    }
    let check = &self.checks[nc.check_index];
    use std::borrow::BorrowMut;
    // no two thread will simultaneously do the same check.
    let result = check.checker.try_lock().unwrap().borrow_mut().borrow_mut().check();
    self.latest_results.write().unwrap()[nc.check_index] = Some(result.clone());
    let log_entry = LogEntry{
      check_index: nc.check_index,
      check_desc: check.desc,
      result: result,
      time: time::SystemTime::now(),
    };
    self.push_log(log_entry);
    self.queue.lock().unwrap().push(NextCheck{
      check_index: nc.check_index,
      scheduled_time: time::Instant::now() + check.min_check_interval
    });
    true
  }

  fn push_log(&self, entry: LogEntry) {
    self.log.lock().unwrap().push(entry);
    self.log_wait.notify_all();
  }

  /// Move the existing log entries into `buf`, clearing the internal log store.
  pub fn read_logs(&self, buf: &mut Vec<LogEntry>) {
    let mut log_store = self.log.lock().unwrap();
    let len = log_store.len();
    buf.extend(log_store.drain(0..len));
  }

  /// Wait for at least one log entry to become available, without consuming the
  /// entry.
  pub fn wait_logs(&self) {
    let mut lg = self.log.lock().unwrap();
    while lg.len() == 0 {
      lg = self.log_wait.wait(lg).unwrap();
    }
  }

  pub fn get_latest_results(&self) -> LatestResultsGuard {
    LatestResultsGuard(self, self.latest_results.read().unwrap())
  }
}

pub struct Check {
  pub checker: Box<dyn Checker + Send + Sync>,
  pub min_check_interval: time::Duration,
  pub desc: &'static str,
}

pub struct InnerCheck {
  checker: Mutex<Box<dyn Checker + Send + Sync>>,
  min_check_interval: time::Duration,
  desc: &'static str,
}

use std::cmp::{PartialEq, PartialOrd, Ordering, Ord, Eq};

struct NextCheck {
  pub check_index: usize,
  pub scheduled_time: time::Instant,
}

impl PartialEq for NextCheck {
  fn eq(&self, other: &Self) -> bool {
    self.scheduled_time == other.scheduled_time
  }
}

impl Eq for NextCheck {}

// Less scheduled_time => higher priority.
impl Ord for NextCheck {
  fn cmp(&self, other: &Self) -> Ordering {
    other.scheduled_time.cmp(&self.scheduled_time)
  }
}

impl PartialOrd for NextCheck {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

pub struct LogEntry {
  pub check_index: usize,
  pub check_desc: &'static str,
  pub result: CheckResult,
  pub time: time::SystemTime,
}

pub struct LatestResultsGuard<'a> (&'a SimpleSchd, RwLockReadGuard<'a, Vec<Option<CheckResult>>>);

use std::ops::Deref;

impl<'a> Deref for LatestResultsGuard<'a> {
  type Target = [Option<CheckResult>];

  fn deref(&self) -> &Self::Target {
    self.1.deref().as_slice()
  }
}

impl<'a> LatestResultsGuard<'a> {
  pub fn get_non_ok_checks(&'a self) -> impl Iterator<Item = (&'static str, &CheckResult)> + 'a {
    self.deref().iter().enumerate().filter_map(move |(i, cr)| {
      if let Some(cr) = cr {
        if cr.result_type == CheckResultType::UP {
          None
        } else {
          Some((self.0.checks[i].desc, cr))
        }
      } else {
        None
      }
    })
  }
}
