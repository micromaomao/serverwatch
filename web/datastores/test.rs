use super::*;
use super::sqlite::SQLiteDataStore;
use serverwatch::checkers::{CheckResult};

use std::time;
fn get_time(t: u64) -> time::SystemTime {
	time::UNIX_EPOCH + time::Duration::from_secs(t)
}

#[test]
fn add_query_search_log() {
	let store = SQLiteDataStore::new_in_memory().unwrap();
	store.search_log(0, LogFilter::default(), LogOrder::Unordered, Box::new(|_, _| panic!("!"))).unwrap();

	let cl_1 = CheckLog{
		time: get_time(1),
		result: CheckResult::up(None),
	};
	let id_1 = store.add_log(0, cl_1.clone()).unwrap();
	let cl_2 = CheckLog{
		time: get_time(2),
		result: CheckResult::error(Some("test test".to_owned())),
	};
	let id_2 = store.add_log(0, cl_2.clone()).unwrap();
	assert_eq!(&store.query_log(id_1).unwrap(), &cl_1);
	assert_eq!(&store.query_log(id_2).unwrap(), &cl_2);

	let cl_3 = CheckLog{
		time: get_time(2),
		result: CheckResult::up(Some("same time insertion".to_owned())),
	};
	let id_3 = store.add_log(0, cl_3.clone()).unwrap();
	assert_eq!(&store.query_log(id_3).unwrap(), &cl_3);
	assert_eq!(&store.query_log(id_2).unwrap(), &cl_2);

	let mut result = vec![];
	store.search_log(0, LogFilter::default(), LogOrder::TimeAsc, Box::new(|check_log_id, check_log| {
		result.push((check_log_id, check_log));
		true
	})).unwrap();
	assert_eq!(result.len(), 3);
	if result[1].0 == id_3 && result[2].0 == id_2 {
		result.swap(1, 2);
	}
	assert_eq!(&result[..], &[
		(id_1, cl_1.clone()),
		(id_2, cl_2.clone()),
		(id_3, cl_3.clone()),
	]);

	result.clear();
	store.search_log(0, LogFilter::default(), LogOrder::TimeDesc, Box::new(|check_log_id, check_log| {
		result.push((check_log_id, check_log));
		true
	})).unwrap();
	if result[0].0 == id_2 && result[1].0 == id_3 {
		result.swap(0, 1);
	}
	assert_eq!(&result[..], &[
		(id_3, cl_3.clone()),
		(id_2, cl_2.clone()),
		(id_1, cl_1.clone()),
	]);

	store.search_log(1, LogFilter::default(), LogOrder::Unordered, Box::new(|_, _| panic!("!"))).unwrap();
	store.search_log(0, LogFilter{include_up: false, include_error: false, ..LogFilter::default()}, LogOrder::Unordered, Box::new(|_, _| panic!("!"))).unwrap();
	store.search_log(0, LogFilter{min_time: Some(get_time(3)), ..LogFilter::default()}, LogOrder::Unordered, Box::new(|_, _| panic!("!"))).unwrap();
	store.search_log(0, LogFilter{max_time: Some(get_time(0)), ..LogFilter::default()}, LogOrder::Unordered, Box::new(|_, _| panic!("!"))).unwrap();

	result.clear();
	store.search_log(0, LogFilter{min_time: Some(get_time(1)), max_time: Some(get_time(2)), ..LogFilter::default()}, LogOrder::TimeAsc, Box::new(|check_log_id, check_log| {
		result.push((check_log_id, check_log));
		true
	})).unwrap();
	assert_eq!(&result[..], &[
		(id_1, cl_1.clone())
	]);

	result.clear();
	store.search_log(0, LogFilter{min_time: Some(get_time(2)), max_time: Some(get_time(3)), ..LogFilter::default()}, LogOrder::TimeAsc, Box::new(|check_log_id, check_log| {
		result.push((check_log_id, check_log));
		true
	})).unwrap();
	if result[0].0 == id_3 && result[1].0 == id_2 {
		result.swap(0, 1);
	}
	assert_eq!(&result[..], &[
		(id_2, cl_2.clone()),
		(id_3, cl_3.clone())
	]);

	store.search_log(0, LogFilter{min_time: Some(get_time(2)), max_time: Some(get_time(2)), ..LogFilter::default()}, LogOrder::Unordered, Box::new(|_, _| panic!("!"))).unwrap();
	store.search_log(0, LogFilter{min_time: Some(get_time(3)), max_time: Some(get_time(2)), ..LogFilter::default()}, LogOrder::Unordered, Box::new(|_, _| panic!("!"))).unwrap();
	store.search_log(0, LogFilter{min_time: Some(get_time(2)), max_time: Some(get_time(0)), ..LogFilter::default()}, LogOrder::Unordered, Box::new(|_, _| panic!("!"))).unwrap();
}

fn get_logcounts(u: u64, w: u64, e: u64) -> LogCounts {
	LogCounts{num_up: u, num_warn: w, num_error: e}
}

fn fromto(from: u64, to: u64) -> LogFilter {
	LogFilter{min_time: Some(get_time(from)), max_time: Some(get_time(to)), ..LogFilter::default()}
}

#[test]
fn count_query() {
	let store = SQLiteDataStore::new_in_memory().unwrap();
	let logs = [
		// 0
		CheckLog{
			time: get_time(100),
			result: CheckResult::up(None),
		},
		// 1
		CheckLog{
			time: get_time(200),
			result: CheckResult::up(None),
		},
		// 2
		CheckLog{
			time: get_time(250),
			result: CheckResult::error(None),
		},
		// 3
		CheckLog{
			time: get_time(300),
			result: CheckResult::warn(None),
		},
		// 4
		CheckLog{
			time: get_time(350),
			result: CheckResult::up(None),
		},
		// 5
		CheckLog{
			time: get_time(400),
			result: CheckResult::up(None),
		},
		// 6
		CheckLog{
			time: get_time(500),
			result: CheckResult::up(None),
		},
		// 7
		CheckLog{
			time: get_time(500),
			result: CheckResult::up(None),
		},
	];
	let mut ids: Vec<CheckLogId> = Vec::new();

	assert_eq!(store.count_logs(0, LogFilter::default()).unwrap(), get_logcounts(0, 0, 0));

	macro_rules! add {
		($i:expr) => {
			ids.push(store.add_log(0, logs[$i].clone()).unwrap())
		};
	}
	macro_rules! test {
		($f:expr, $t:expr, $u:expr, $w:expr, $e:expr) => {
			assert_eq!(store.count_logs(0, fromto($f, $t)).unwrap(), get_logcounts($u, $w, $e));
		};
	}

	add!(0);
	assert_eq!(store.count_logs(0, LogFilter::default()).unwrap(), get_logcounts(1, 0, 0));
	test!(0, 200, 1, 0, 0);
	test!(100, 200, 1, 0, 0);
	test!(101, 200, 0, 0, 0);
	test!(100, 100, 0, 0, 0);
	test!(99, 100, 0, 0, 0);
	test!(99, 101, 1, 0, 0);
	test!(101, 99, 0, 0, 0);

	add!(1);
	test!(0, 200, 1, 0, 0);
	test!(0, 300, 2, 0, 0);

	add!(2);
	test!(0, 250, 2, 0, 0);
	test!(0, 280, 2, 0, 1);
	test!(250, 280, 0, 0, 1);
	test!(250, 250, 0, 0, 0);

	add!(3);
	test!(0, 400, 2, 1, 1);
	assert_eq!(store.count_logs(0, LogFilter{min_time: Some(get_time(300)), ..LogFilter::default()}).unwrap(), get_logcounts(0, 1, 0));
	assert_eq!(store.count_logs(0, LogFilter{min_time: Some(get_time(200)), ..LogFilter::default()}).unwrap(), get_logcounts(1, 1, 1));
	assert_eq!(store.count_logs(0, LogFilter::default()).unwrap(), get_logcounts(2, 1, 1));

	for i in 4..=6 {
		add!(i);
	}
	test!(0, 500, 4, 1, 1);
	assert_eq!(store.count_logs(0, LogFilter::default()).unwrap(), get_logcounts(5, 1, 1));

	add!(7);
	assert_eq!(store.count_logs(0, LogFilter::default()).unwrap(), get_logcounts(6, 1, 1));
	test!(500, 600, 2, 0, 0);
	assert_eq!(store.count_logs(0, LogFilter::after(get_time(500))).unwrap(), get_logcounts(2, 0, 0));
	assert_eq!(store.count_logs(0, LogFilter::after(get_time(400))).unwrap(), get_logcounts(3, 0, 0));
	assert_eq!(store.count_logs(0, LogFilter::after(get_time(300))).unwrap(), get_logcounts(4, 1, 0));
	assert_eq!(store.count_logs(0, LogFilter::after(get_time(299))).unwrap(), get_logcounts(4, 1, 0));
}
