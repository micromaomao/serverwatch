use std::time;
use std::thread;
use std::sync::mpsc;

/// Run a closure with a timeout, returning either its return value, or None, in case
/// it timed out.
///
/// This function should only take at most `timeout` to return.
///
/// ## Example
/// ```rust
/// # use serverwatch::utils::with_timeout;
/// use std::time::Duration;
/// use std::thread::sleep;
/// assert_eq!(with_timeout(|| {sleep(Duration::from_millis(1000)); 1}, Duration::from_millis(500)), None);
/// assert_eq!(with_timeout(|| {sleep(Duration::from_millis(1000)); 1}, Duration::from_millis(1500)), Some(1));
/// ```
pub fn with_timeout<F: FnOnce() -> R + Send + 'static, R: Send + 'static>(f: F, timeout: time::Duration) -> Option<R> {
	let (sender, recv) = mpsc::sync_channel(0);
	thread::spawn(move || {
		let _ = sender.send(f());
	});
	match recv.recv_timeout(timeout) {
		Err(e) => {
			if e == mpsc::RecvTimeoutError::Timeout {
				None
			} else {
				panic!("sender disconnected?")
			}
		}
		Ok(r) => Some(r)
	}
}

#[test]
fn with_timeout_test() {
	assert_eq!(with_timeout(|| {1}, time::Duration::from_secs(1)), Some(1));
	assert_eq!(with_timeout(|| {thread::sleep(time::Duration::from_millis(500)); 1}, time::Duration::from_secs(1)), Some(1));
	let measure = time::Instant::now();
	assert_eq!(with_timeout(|| {thread::sleep(time::Duration::from_millis(1500)); 1}, time::Duration::from_secs(1)), None);
	assert!(measure.elapsed() < time::Duration::from_millis(1200));
}
