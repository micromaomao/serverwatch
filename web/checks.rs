use serverwatch::checkers::{http::HttpChecker, tls::{CertificateChecker, CertificateCheckerStartTLSOptions}};
use std::time::Duration;
use serverwatch::scheduler::simple_schd::Check;

fn s_string(s: String) -> &'static str {
  Box::leak(Box::new(s))
}

pub fn get_checks() -> Vec<Check> {
  let mut list = Vec::new();
  let domains: Vec<(&'static str, u16, Option<&'static str>)> = vec![
    ("maowtm.org", 200, Some("Mao Wtm")),
    ("paper.sc", 200, Some("search engine for CIE papers")),
    ("static.maowtm.org", 404, Some("https://static.maowtm.org/svg/logo.svg")),
    ("death.maowtm.org", 200, Some("<!DOCTYPE HTML>")),
    ("oa.szlf.com", 302, None),
    ("status.maowtm.org", 200, Some("HTTP maowtm.org"))
  ];
  for (domain, expect_status, expect_contains) in domains.into_iter() {
    list.push(Check{
      desc: s_string(format!("HTTP {}", domain)),
      checker: {
        let mut c = HttpChecker::new(&format!("https://{}/", domain)).unwrap();
        c.set_timeouts(Duration::from_secs(1), Duration::from_secs(5));
        c.expect_status(expect_status);
        if let Some(f) = expect_contains {
          c.expect_response_contains(f);
        }
        Box::new(c)
      },
      min_check_interval: Duration::from_secs(30)
    });
    list.push(Check{
      desc: s_string(format!("TLS {}", domain)),
      checker: {
        let mut c = CertificateChecker::builder(domain.to_owned(), 443);
        c.set_expiry_threshold(Duration::from_secs(20*24*60*60)); // 20 days
        Box::new(c.build().unwrap())
      },
      min_check_interval: Duration::from_secs(60*10)
    });
  }
  list.push(Check{
    desc: "https://paper.sc/search/?as=json&query=test",
    checker: {
      let mut c = HttpChecker::new("https://paper.sc/search/?as=json&query=test").unwrap();
      c.set_timeouts(Duration::from_secs(1), Duration::from_secs(5));
      c.expect_200();
      c.expect_response_contains(r#"{"response":"text","list""#);
      Box::new(c)
    },
    min_check_interval: Duration::from_secs(60)
  });
  list
}
