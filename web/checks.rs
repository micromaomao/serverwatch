use serverwatch::checkers::{http::HttpChecker, tls::CertificateChecker};
use std::time::Duration;
use serverwatch::scheduler::simple_schd::Check;

fn s_string(s: String) -> &'static str {
  Box::leak(Box::new(s))
}

pub fn get_checks() -> Vec<Check> {
  let mut list = Vec::new();
  for domain in vec!["maowtm.org", "paper.sc", "static.maowtm.org", "localhost"].into_iter() {
    list.push(Check{
      desc: s_string(format!("HTTP {}", domain)),
      checker: {
        let mut c = HttpChecker::new(&format!("https://{}/", domain)).unwrap();
        c.set_timeouts(Duration::from_secs(1), Duration::from_secs(5));
        Box::new(c)
      },
      min_check_interval: Duration::from_secs(10)
    });
    list.push(Check{
      desc: s_string(format!("TLS {}", domain)),
      checker: {
        let mut c = CertificateChecker::builder(domain.to_owned(), 443);
        c.set_expiry_threshold(Duration::from_secs(10*24*60*60)); // 10 days
        Box::new(c.build().unwrap())
      },
      min_check_interval: Duration::from_secs(60)
    });
  }
  list
}
