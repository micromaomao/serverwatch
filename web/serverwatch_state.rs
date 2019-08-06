use serverwatch::scheduler::simple_schd::SimpleSchd;
use serverwatch::checkers::CheckResult;
use super::checks;
use std::collections::VecDeque;
use std::sync::RwLock;

pub const NUM_STORED_RESULTS: usize = 128;

pub fn init() -> SwState {
  let checks_list = checks::get_checks();
  let n_checks = checks_list.len();
  let descs_list: Vec<&'static str> = checks_list.iter().map(|x| x.desc).collect();
  let schd: &'static _ = Box::leak(Box::new(SimpleSchd::new(checks_list)));
  for _ in 0..4 {
    std::thread::spawn(move || {
      while schd.step() {}
    });
  }
  let result_store: &'static RwLock<Vec<VecDeque<(CheckResult, std::time::SystemTime)>>> = Box::leak(Box::new(
    RwLock::new({
      let mut r = Vec::new();
      r.reserve_exact(n_checks);
      for _ in 0..n_checks {
        r.push(VecDeque::new());
      }
      r
    })
  ));
  std::thread::spawn(move || {
    loop {
      let mut logs = Vec::new();
      schd.read_logs(&mut logs);
      if logs.len() > 0 {
        let mut result_store = result_store.write().unwrap();
        for log in logs.into_iter() {
          let deque = &mut result_store[log.check_index];
          deque.push_front((log.result, log.time));
          if deque.len() > NUM_STORED_RESULTS {
            deque.truncate(NUM_STORED_RESULTS);
          }
        }
      }
      schd.wait_logs();
    }
  });
  SwState{
    schd,
    descs_list,
    result_store,
  }
}

pub struct SwState {
  pub schd: &'static SimpleSchd,
  pub descs_list: Vec<&'static str>,
  pub result_store: &'static RwLock<Vec<VecDeque<(CheckResult, std::time::SystemTime)>>>,
}
