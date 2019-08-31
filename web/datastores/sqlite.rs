use super::*;
use rusqlite;
use rusqlite::OptionalExtension;
use std::sync::Mutex;
use std::cell::RefCell;
use std::time::{SystemTime, Duration};
use serverwatch::checkers::{CheckResult, CheckResultType};
use rusqlite::types::{Value, ValueRef};

pub struct SQLiteDataStore {
  conn: Mutex<RefCell<rusqlite::Connection>>,
}

impl SQLiteDataStore {
  pub fn new_in_memory() -> DataResult<Self> {
    let conn = rusqlite::Connection::open_in_memory().map_err(DatabaseError::from_inner)?;
    Self::initialize(&conn)?;
    Ok(Self{conn: Mutex::new(RefCell::new(conn))})
  }
  pub fn open<P: AsRef<std::path::Path>>(path: P) -> DataResult<Self> {
    let conn = rusqlite::Connection::open(path).map_err(DatabaseError::from_inner)?;
    if conn.query_row(r#"SELECT 1 FROM sqlite_master WHERE type="table" AND name="metadata""#, rusqlite::NO_PARAMS, |_| {Ok(true)}).optional().map_err(DatabaseError::from_inner)?.is_none() {
      Self::initialize(&conn)?;
    }
    Self::check(&conn)?;
    conn.busy_timeout(Duration::from_millis(100)).map_err(|e| DatabaseError::from_inner_and_str(e, "Unable to set busy timeout"))?;
    Ok(Self{conn: Mutex::new(RefCell::new(conn))})
  }

  fn initialize(conn: &rusqlite::Connection) -> DataResult<()> {
    conn.execute_batch(include_str!("scheme.sql")).map_err(DatabaseError::from_inner)
  }

  fn check(conn: &rusqlite::Connection) -> DataResult<()> {
    let version: Option<String> = conn.query_row(r#"SELECT value FROM metadata WHERE name="version""#, rusqlite::NO_PARAMS, |row| {row.get(0)}).optional().map_err(DatabaseError::from_inner)?;
    match version {
      None => Err(DatabaseError::from_static_str("no version field in metadata?")),
      Some(ver) => {
        if ver != "0" {
          Err(DatabaseError::from_string(format!("Invalid version: {}", ver)))
        } else {
          Ok(())
        }
      }
    }
  }
}

fn time2int(time: SystemTime) -> i64 {
  let epoch = std::time::UNIX_EPOCH;
  if time >= epoch {
    time.duration_since(epoch).unwrap().as_millis() as i64
  } else {
    epoch.duration_since(time).unwrap().as_millis() as i64
  }
}
fn int2time(i: i64) -> time::SystemTime {
  let epoch = std::time::UNIX_EPOCH;
  if i >= 0 {
    epoch + Duration::from_millis(i as u64)
  } else {
    epoch - Duration::from_millis((-i) as u64)
  }
}

#[test]
fn time2int_test() {
  assert_eq!(time2int(SystemTime::UNIX_EPOCH), 0);
  assert_eq!(time2int(SystemTime::UNIX_EPOCH + Duration::from_secs(1)), 1000);
  assert_eq!(time2int(SystemTime::UNIX_EPOCH - Duration::from_secs(1)), -1000);
}

pub fn row_to_check_log(row: &rusqlite::Row) -> rusqlite::Result<DataResult<CheckLog>> {
  Ok(Ok(CheckLog{
    time: int2time(row.get(0)?),
    result: CheckResult{
      result_type: match str_to_result_type(&(row.get(1)?: String)) { Some(s) => s, None => return Ok(Err(DatabaseError::from_static_str("Invalid enum value for result_type"))) },
      info: match row.get_raw_checked(2)? {
        ValueRef::Null => None,
        ValueRef::Text(s) => Some(match String::from_utf8(Vec::from(s)) {
          Ok(s) => s,
          Err(e) => return Ok(Err(DatabaseError::from_inner_and_str(e, "UTF8 decoding error when getting info"))),
        }),
        _ => return Ok(Err(DatabaseError::from_static_str("invalid column type for info"))),
      }
    }
  }))
}

impl DataStore for SQLiteDataStore {
  fn add_log_and_push<'a>(&self, check_id: CheckId, log: CheckLog, mut send_push: Box<dyn FnMut(String, Vec<u8>, Vec<u8>) + 'a>) -> DataResult<CheckLogId> {
    let now = log.time;
    let conn = self.conn.lock().unwrap();
    let mut conn = conn.borrow_mut();
    let mut tr = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| DatabaseError::from_inner_and_str(e, "unable to start transaction"))?;
    tr.set_drop_behavior(rusqlite::DropBehavior::Rollback);
    tr.prepare_cached(r#"INSERT INTO Logs ("check_id", "time", "result_type", "result_info") VALUES (?, ?, ?, ?);"#).map_err(DatabaseError::from_inner)?
      .execute(&[Value::from(check_id), Value::from(time2int(log.time)), Value::from(result_type_to_str(log.result.result_type).to_owned()), match log.result.info { Some(ref s) => Value::from(s.to_owned()), None => Value::Null }]).map_err(DatabaseError::from_inner)?;
    let log_id = tr.last_insert_rowid() as u64;
    let last_counts: Option<(i64, i64, i64, SystemTime)> = tr.prepare_cached("SELECT count_up, count_warn, count_error, up_to FROM LogCount WHERE check_id = ? ORDER BY up_to DESC LIMIT 1").map_err(DatabaseError::from_inner)?.query_row(&[check_id],
          |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, int2time(row.get(3)?)))
          }).optional().map_err(DatabaseError::from_inner)?;
    if let Some((mut count_up, mut count_warn, mut count_error, up_to)) = last_counts {
      let result_type = log.result.result_type;
      match result_type {
        CheckResultType::UP => count_up += 1,
        CheckResultType::WARN => count_warn += 1,
        CheckResultType::ERROR => count_error += 1,
      }
      if up_to < now {
        tr.prepare_cached(r#"INSERT INTO LogCount ("check_id", "up_to", "count_up", "count_warn", "count_error") VALUES (?, ?, ?, ?, ?);"#).map_err(DatabaseError::from_inner)?
          .execute(&[Value::from(check_id), Value::from(time2int(now)), Value::from(count_up), Value::from(count_warn), Value::from(count_error)]).map_err(|e| DatabaseError::from_inner_and_str(e, "unable to insert new counts"))?;
      } else {
        tr.prepare_cached(r#"UPDATE LogCount SET count_up = ?, count_warn = ?, count_error = ? WHERE up_to = ?;"#).map_err(DatabaseError::from_inner)?
          .execute(&[Value::from(count_up), Value::from(count_warn), Value::from(count_error), Value::from(time2int(up_to))]).map_err(|e| DatabaseError::from_inner_and_str(e, "unable to update counts"))?;
      }
    } else {
      let mut count_up = 0i64;
      let mut count_warn = 0i64;
      let mut count_error = 0i64;
      let result_type = log.result.result_type;
      match result_type {
        CheckResultType::UP => count_up += 1,
        CheckResultType::WARN => count_warn += 1,
        CheckResultType::ERROR => count_error += 1,
      }
      tr.prepare_cached(r#"INSERT INTO LogCount ("check_id", "up_to", "count_up", "count_warn", "count_error") VALUES (?, ?, ?, ?, ?);"#).map_err(DatabaseError::from_inner)?
        .execute(&[Value::from(check_id), Value::from(time2int(now)), Value::from(count_up), Value::from(count_warn), Value::from(count_error)]).map_err(|e| DatabaseError::from_inner_and_str(e, "unable to insert new counts"))?;
    }

    if log.result.result_type != CheckResultType::UP {
      tr.prepare_cached(r#"SELECT endpoint_url, auth, client_p256dh, notify_warn FROM pushSubscriptions WHERE check_id = ?"#).map_err(DatabaseError::from_inner)?
        .query_and_then::<_, rusqlite::Error, _, _>(&[check_id], |row| {
          let endpoint_url: String = row.get(0)?;
          let auth: Vec<u8> = row.get(1)?;
          let p256dh: Vec<u8> = row.get(2)?;
          let notify_warn: bool = row.get(3)?;
          if !notify_warn && log.result.result_type == CheckResultType::WARN {
            return Ok(());
          }
          send_push(endpoint_url, auth, p256dh);
          return Ok(());
        }).map_err(DatabaseError::from_inner)?.count();
    }

    tr.commit().map_err(|e| DatabaseError::from_inner_and_str(e, "unable to commit transaction"))?;

    Ok(log_id)
  }
  fn query_log(&self, id: CheckLogId) -> DataResult<CheckLog> {
    let conn = self.conn.lock().unwrap();
    let conn = conn.borrow();
    conn.query_row("SELECT time, result_type, result_info FROM Logs WHERE id = ?", &[id as i64], row_to_check_log).map_err(DatabaseError::from_inner)?
  }
  fn search_log<'a>(&'a self, check: CheckId, search: LogFilter, order: LogOrder, mut each_fn: Box<dyn FnMut(CheckLogId, CheckLog) -> bool + 'a>) -> DataResult<()> {
    let mut sql = String::from("SELECT time, result_type, result_info, id FROM Logs WHERE check_id = ?");
    let mut values: Vec<Value> = vec![Value::from(check)];
    if let Some(min_time) = search.min_time {
      sql.push_str(" AND time >= ? ");
      values.push(Value::from(time2int(min_time)));
    }
    if let Some(max_time) = search.max_time {
      sql.push_str(" AND time < ?");
      values.push(Value::from(time2int(max_time)));
    }
    if !search.include_up {
      sql.push_str(r#" AND result_type != "up""#);
    }
    if !search.include_warn {
      sql.push_str(r#" AND result_type != "warn""#);
    }
    if !search.include_error {
      sql.push_str(r#" AND result_type != "error""#);
    }
    match order {
      LogOrder::Unordered => {},
      LogOrder::TimeAsc => {
        sql.push_str(" ORDER BY time ASC");
      },
      LogOrder::TimeDesc => {
        sql.push_str(" ORDER BY time DESC");
      }
    }
    let conn = self.conn.lock().unwrap();
    let conn = conn.borrow();
    let mut stat = conn.prepare_cached(&sql).map_err(|e| DatabaseError::from_inner_and_str(e, "unable to prepare SQL"))?;
    for r in rusqlite::Statement::query_and_then::<DataResult<(CheckLogId, CheckLog)>, rusqlite::Error, &[Value], _>(&mut *stat, &values, |row| {
      let id = row.get(3)?: i64 as CheckLogId;
      let check_log = row_to_check_log(row)?;
      Ok(match check_log {
        Ok(log) => Ok((id, log)),
        Err(e) => Err(e)
      })
    }).map_err(DatabaseError::from_inner)?.map(|x| {
      match x {
        Ok(y) => y,
        Err(e) => Err(DatabaseError::from_inner(e))
      }
    }) {
      if let Ok((id, log)) = r {
        if !each_fn(id, log) {
          break;
        }
      } else {
        return Err(r.unwrap_err());
      }
    }
    Ok(())
  }
  fn count_logs(&self, check: CheckId, filter: LogFilter) -> DataResult<LogCounts> {
    let conn = self.conn.lock().unwrap();
    let conn = conn.borrow();
    let mut stat = String::from("SELECT up_to, count_up, count_warn, count_error FROM LogCount WHERE check_id = ?");
    let mut vals = vec![Value::from(check)];
    if let Some(max_time) = filter.max_time {
      stat.push_str(" AND up_to < ?");
      vals.push(Value::from(time2int(max_time)));
    }
    if let Some(min_time) = filter.min_time {
      stat.push_str(" AND up_to >= ?");
      vals.push(Value::from(time2int(min_time)));
    }
    let base_stat = stat.clone();
    stat.push_str(" ORDER BY up_to DESC LIMIT 1");
    let mut stat = conn.prepare_cached(&stat).map_err(DatabaseError::from_inner)?;
    let last: Option<(i64, i64, i64, i64)> = stat.query_row(&vals, |row| {
      Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    }).optional().map_err(DatabaseError::from_inner)?;
    if last.is_none() {
      return Self::select_count_logs(&*conn, check, filter.min_time.map(time2int), true, filter.max_time.map(time2int), false);
    }
    let (last_up_to, num_up, num_warn, num_error) = last.unwrap();
    if filter.min_time.is_none() {
      return Ok(LogCounts{num_up: num_up as u64, num_warn: num_warn as u64, num_error: num_error as u64} + Self::select_count_logs(&*conn, check, Some(last_up_to), false, filter.max_time.map(time2int), false)?);
    }
    let mut stat = base_stat;
    stat.push_str(" ORDER BY up_to ASC LIMIT 1");
    let mut stat = conn.prepare_cached(&stat).map_err(DatabaseError::from_inner)?;
    let (first_up_to, first_num_up, first_num_warn, first_num_error): (i64, i64, i64, i64) = stat.query_row(&vals, |row| {
      Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    }).map_err(DatabaseError::from_inner)?;
    if first_up_to == last_up_to {
      return Self::select_count_logs(&*conn, check, filter.min_time.map(time2int), true, filter.max_time.map(time2int), false);
    }
    let mut res = LogCounts{
      num_up: (num_up - first_num_up) as u64, num_warn: (num_warn - first_num_warn) as u64, num_error: (num_error - first_num_error) as u64,
    };
    res += Self::select_count_logs(&*conn, check, filter.min_time.map(time2int), true, Some(first_up_to), true)?;
    res += Self::select_count_logs(&*conn, check, Some(last_up_to), false, filter.max_time.map(time2int), false)?;
    Ok(res)
  }

  fn update_push_subscriptions(&self, endpoint_url: &str, auth: &[u8], client_p256dh: &[u8], list: &[PushSubscription]) -> DataResult<()> {
    let conn = self.conn.lock().unwrap();
    let mut conn = conn.borrow_mut();
    let mut tr = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| DatabaseError::from_inner_and_str(e, "unable to start transaction"))?;
    tr.set_drop_behavior(rusqlite::DropBehavior::Rollback);

    tr.prepare_cached("DELETE FROM pushSubscriptions WHERE endpoint_url = ? AND auth = ?").map_err(DatabaseError::from_inner)?
      .execute(&[Value::from(endpoint_url.to_owned()), Value::from(auth.to_owned())]).map_err(|e| DatabaseError::from_inner_and_str(e, "Unable to delete existing rows"))?;

    let mut insert_stat = tr.prepare_cached(r#"INSERT INTO pushSubscriptions ("endpoint_url", "check_id", "auth", "client_p256dh", "notify_warn") VALUES (?, ?, ?, ?, ?);"#).map_err(DatabaseError::from_inner)?;
    for sub in list {
      insert_stat.execute(&[Value::from(endpoint_url.to_owned()), Value::from(sub.check_id), Value::from(auth.to_owned()), Value::from(client_p256dh.to_owned()), Value::from(sub.notify_warn)]).map_err(DatabaseError::from_inner)?;
    }

    std::mem::drop(insert_stat);
    tr.commit().map_err(|e| DatabaseError::from_inner_and_str(e, "unable to commit transaction"))?;

    Ok(())
  }
}

impl SQLiteDataStore {
  fn select_count_logs(conn: &rusqlite::Connection, check: CheckId, from: Option<i64>, include_from: bool, to: Option<i64>, include_to: bool) -> DataResult<LogCounts> {
    let mut stat = String::from("SELECT result_type, count() FROM Logs WHERE check_id = ?");
    let mut vals = vec![Value::from(check)];
    if let Some(from) = from {
      stat.push_str(" AND time ");
      if include_from {
        stat.push_str(">=");
      } else {
        stat.push_str(">");
      }
      stat.push_str(" ?");
      vals.push(Value::from(from));
    }
    if let Some(to) = to {
      stat.push_str(" AND time ");
      if include_to {
        stat.push_str("<=");
      } else {
        stat.push_str("<");
      }
      stat.push_str(" ?");
      vals.push(Value::from(to));
    }
    stat.push_str(" GROUP BY result_type");
    let mut stat = conn.prepare_cached(&stat).map_err(DatabaseError::from_inner)?;
    let mut num_up = 0u64;
    let mut num_warn = 0u64;
    let mut num_error = 0u64;
    for r in stat.query_and_then(&vals, |row| {
      let result_type: String = row.get(0)?;
      let count: i64 = row.get(1)?;
      match &result_type[..] {
        "up" => num_up += count as u64,
        "warn" => num_warn += count as u64,
        "error" => num_error += count as u64,
        _ => return Ok(Err(DatabaseError::from_static_str("Invalid enum value")))
      }
      Ok(Ok(()))
    }).map_err(DatabaseError::from_inner)? {
      (r: rusqlite::Result<DataResult<()>>).map_err(DatabaseError::from_inner)??;
    }
    Ok(LogCounts{num_up, num_warn, num_error})
  }
}
