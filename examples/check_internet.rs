use serverwatch::scheduler::simple_schd::{SimpleSchd, Check};
use serverwatch::checkers::http::HttpChecker;
use serverwatch::checkers::tls::CertificateChecker;
use serverwatch::checkers::CheckResultType;
use std::time::Duration;

fn main () {
  let schd = SimpleSchd::new(vec![
    // Google
    Check{
      desc: "HTTP OK: https://www.google.com/",
      checker: {
        let mut c = HttpChecker::new("https://www.google.com/").unwrap();
        c.set_timeouts(Duration::from_secs(1), Duration::from_secs(10));
        c.expect_200();
        c.expect_response_contains("I'm Feeling Lucky");
        Box::new(c)
      },
      min_check_interval: Duration::from_secs(10),
    },
    Check{
      desc: "TLS certificate not close to expiry: google.com",
      checker: Box::new(CertificateChecker::builder("google.com".to_owned(), 443).build().unwrap()),
      min_check_interval: Duration::from_secs(30),
    },

    // Github
    Check{
      desc: "HTTP OK: https://github.com/",
      checker: {
        let mut c = HttpChecker::new("https://github.com/").unwrap();
        c.set_timeouts(Duration::from_secs(1), Duration::from_secs(10));
        c.expect_200();
        c.expect_response_contains("GitHub is where people build software.");
        Box::new(c)
      },
      min_check_interval: Duration::from_secs(10),
    },
    Check{
      desc: "TLS certificate not close to expiry: github.com",
      checker: Box::new(CertificateChecker::builder("github.com".to_owned(), 443).build().unwrap()),
      min_check_interval: Duration::from_secs(30),
    },

    // Cloudflare
    Check{
      desc: "HTTP OK: static.maowtm.org (i.e. cloudflare)",
      checker: {
        let mut c = HttpChecker::new("https://static.maowtm.org/svg/logo.svg").unwrap();
        c.set_timeouts(Duration::from_secs(1), Duration::from_secs(10));
        c.expect_200();
        c.expect_response_contains("svg xmlns:dc");
        Box::new(c)
      },
      min_check_interval: Duration::from_secs(10),
    },
    Check{
      desc: "TLS certificate not close to expiry: static.maowtm.org",
      checker: Box::new(CertificateChecker::builder("static.maowtm.org".to_owned(), 443).build().unwrap()),
      min_check_interval: Duration::from_secs(30),
    },
  ]);
  let schd: &'static _ = Box::leak(Box::new(schd)); // No scoped thread in std.
  for _ in 0..5 {
    std::thread::spawn(move || {
      loop {
        if !schd.step() {
          return;
        }
      }
    });
  }
  loop {
    let mut logs = Vec::new();
    schd.read_logs(&mut logs);
    for i in logs.iter() {
      eprintln!("\x1b[2K\rCheck \x1b[36m{:?}\x1b[0m: {} {}", i.check_desc, match i.result.result_type {
        CheckResultType::UP => "\x1b[32mUP\x1b[0m",
        CheckResultType::WARN => "\x1b[33mWARN\x1b[0m",
        CheckResultType::ERROR => "\x1b[31mDOWN\x1b[0m",
      }, i.result.info.as_ref().map(|x| x.as_str()).unwrap_or("(no info)"));
    }
    let failed_checks = schd.get_latest_results().get_non_ok_checks().map(|x| x.0).collect::<Vec<&'static str>>();
    if failed_checks.len() > 0 {
      eprint!("\r\x1b[46;97m\x1b[2KStatus: {} checks failed: {}\x1b[0m\r", failed_checks.len(), failed_checks.join(", "));
    } else {
      eprint!("\r\x1b[42;97m\x1b[2KStatus: Everything's OK!\x1b[0m\r");
    }
    schd.wait_logs();
  }
}
