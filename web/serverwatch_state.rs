use serverwatch::scheduler::simple_schd::SimpleSchd;
use super::{checks, datastores};
use datastores::DataStore;
use std::sync::Arc;

pub struct SwState {
  pub schd: Arc<SimpleSchd>,
  pub descs_list: Vec<&'static str>,
  pub checkids_list: Vec<datastores::CheckId>,
  pub data_store: Arc<datastores::sqlite::SQLiteDataStore>,
}

pub fn init() -> SwState {
  let checks_list = checks::get_checks();
  let descs_list: Vec<&'static str> = checks_list.iter().map(|x| x.desc).collect();
  let checkids_list: Vec<datastores::CheckId> = checks_list.iter().map(|x| x.index).collect();
  let schd = Arc::new(SimpleSchd::new(checks_list.into_iter().map(|x| x.schd_check).collect()));
  let data_store = Arc::new(datastores::sqlite::SQLiteDataStore::open("/tmp/test.db").unwrap());
  for _ in 0..4 {
    let schd_ref = schd.clone();
    std::thread::spawn(move || {
      while schd_ref.step() {}
    });
  }
  let schd_ref = schd.clone();
  {
    let checkids_list = checkids_list.clone();
    let data_store = data_store.clone();
    std::thread::spawn(move || {
      loop {
        let mut logs = Vec::new();
        schd_ref.read_logs(&mut logs);
        if logs.len() > 0 {
          for log in logs.into_iter() {
            let mut try_count = 0u8;
            loop {
              let check_id = checkids_list[log.check_index];
              if let Err(e) = data_store.add_log(check_id, datastores::CheckLog{
                time: log.time, result: log.result.clone()
              }) {
                if try_count < 3 {
                  try_count += 1;
                  continue;
                } else {
                  panic!("Error? Aww man! {}", e); // TODO: Better error handling
                }
              } else {
                break;
              }
            }
          }
        }
        schd_ref.wait_logs();
      }
    });
  }
  SwState{
    schd,
    descs_list,
    data_store,
    checkids_list,
  }
}
