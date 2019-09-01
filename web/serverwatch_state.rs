use serverwatch::scheduler::simple_schd::SimpleSchd;
use super::{checks, datastores, push::push};
use datastores::DataStore;
use std::sync::Arc;
use openssl::ec;

pub struct SwState {
  pub schd: Arc<SimpleSchd>,
  pub descs_list: Vec<&'static str>,
  pub checkids_list: Vec<datastores::CheckId>,
  pub data_store: Arc<dyn datastores::DataStore>,
  pub app_server_key: ec::EcKey<openssl::pkey::Private>,
  pub app_server_pub_key_b64: String,
  pub web_push_reqwest_client: reqwest::Client,
}

pub fn init() -> SwState {
  let checks_list = checks::get_checks();
  let descs_list: Vec<&'static str> = checks_list.iter().map(|x| x.desc).collect();
  let checkids_list: Vec<datastores::CheckId> = checks_list.iter().map(|x| x.index).collect();
  let schd = Arc::new(SimpleSchd::new(checks_list.into_iter().map(|x| x.schd_check).collect()));
  let data_store = Arc::new(datastores::sqlite::SQLiteDataStore::open("./database.db").unwrap());
  for _ in 0..4 {
    let schd_ref = schd.clone();
    std::thread::spawn(move || {
      while schd_ref.step() {}
    });
  }
  let app_server_key = ec::EcKey::private_key_from_pem(include_bytes!("./keys/app_server.key")).unwrap();
  app_server_key.check_key().unwrap();
  let schd_ref = schd.clone();
  let (push_queue_send, push_queue_recv) = std::sync::mpsc::channel();
  {
    let checkids_list = checkids_list.clone();
    let data_store = data_store.clone();
    let descs_list = descs_list.clone();
    std::thread::spawn(move || {
      loop {
        let mut logs = Vec::new();
        schd_ref.read_logs(&mut logs);
        if logs.len() > 0 {
          for log in logs.into_iter() {
            let mut try_count = 0u8;
            loop {
              let check_id = checkids_list[log.check_index];
              let desc = descs_list[log.check_index];
              if let Err(e) = data_store.add_log_and_push(check_id, datastores::CheckLog{
                time: log.time, result: log.result.clone()
              }, Box::new(|endpoint_url: String, auth: Vec<u8>, p256dh: Vec<u8>| {
                  let mut push_body = String::new();
                  push_body.push_str(&format!("{}\n", check_id));
                  push_body.push_str(&format!("{}\n", log.time.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
                  push_body.push_str(&format!("{:?} {:?}ed\n", desc, log.result.result_type));
                  push_body.push_str(match log.result.info {
                    Some(ref info) => info,
                    None => "(no info)"
                  });
                  let _ = push_queue_send.send((endpoint_url, p256dh, auth, push_body, std::time::Duration::from_secs(24*60*60)));
                })) {
                if try_count < 100 {
                  try_count += 1;
                  std::thread::yield_now();
                  continue;
                } else {
                  eprintln!("{}", e);
                  std::process::exit(1);
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
  {
    let push_http_client = reqwest::Client::new();
    let app_server_key = app_server_key.clone();
    let data_store = data_store.clone();
    std::thread::spawn(move || {
      loop {
        let task = match push_queue_recv.recv() {
          Ok(t) => t,
          Err(_) => return,
        };
        if let Err(e) = push(&push_http_client, &app_server_key, &task.0, &task.1, &task.2, task.3.as_bytes(), task.4) {
          eprint!("Push error: endpoint={}: {}", &task.0, &e);
          if e.starts_with("Push endpoint responsed with") {
            let _ = data_store.update_push_subscriptions(&task.0, &task.2, &task.1, &[]);
          }
        }
      }
    });
  }
  let pub_key = app_server_key.public_key();
  let pub_key_bytes = pub_key.to_bytes(app_server_key.group(), openssl::ec::PointConversionForm::UNCOMPRESSED, &mut openssl::bn::BigNumContext::new().unwrap()).unwrap();
  let pub_key_b64 = base64::encode_config(&pub_key_bytes, base64::Config::new(base64::CharacterSet::UrlSafe, false));
  let web_push_reqwest_client = reqwest::Client::new();
  SwState{
    schd,
    descs_list,
    data_store,
    checkids_list,
    app_server_key,
    app_server_pub_key_b64: pub_key_b64,
    web_push_reqwest_client,
  }
}
