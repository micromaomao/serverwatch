use serverwatch::checkers::{http::HttpChecker, tls::{CertificateChecker, CertificateCheckerStartTLSOptions}};
use std::time::Duration;
use serverwatch::scheduler::simple_schd;
use crate::datastores::CheckId;

pub struct Check {
  pub index: CheckId,
  pub desc: &'static str,
  pub schd_check: simple_schd::Check,
}

pub fn get_checks() -> Vec<Check> {
  let mut list = Vec::new();
  macro_rules! add_check {
    ($index:expr, $desc:expr, $min_check_interval:expr, $checker:expr) => {
      list.push(Check{
        index: $index,
        desc: $desc,
        schd_check: simple_schd::Check{
          desc: $desc,
          checker: $checker,
          min_check_interval: $min_check_interval
        }
      });
    };
  }
  macro_rules! http {
    ($index:expr, $domain:expr, $expect_status:expr, $expect_contains:expr, $warn_timeout:expr) => {
      add_check!($index, concat!("HTTP ", $domain), Duration::from_secs(30), {
        let mut c = HttpChecker::new(concat!("https://", $domain, "/")).unwrap();
        c.set_timeouts($warn_timeout, Duration::from_secs(5));
        c.expect_status($expect_status);
        if let Some(f) = $expect_contains {
          c.expect_response_contains(f);
        }
        Box::new(c)
      });
    };
    ($index:expr, $domain:expr, $expect_status:expr, $expect_contains:expr) => {
      http!($index, $domain, $expect_status, $expect_contains, Duration::from_secs(1))
    };
  }
  macro_rules! tls {
    ($index:expr, $domain:expr) => {
      add_check!($index, concat!("TLS ", $domain), Duration::from_secs(600), {
        let mut c = CertificateChecker::builder($domain.to_owned(), 443);
        c.set_expiry_threshold(Duration::from_secs(20*24*60*60)); // 20 days
        Box::new(c.build().unwrap())
      })
    };
  }
  macro_rules! smtp {
    ($index:expr, $domain:expr) => {
      add_check!($index, concat!("SMTP ", $domain), Duration::from_secs(30), {
        let mut c = CertificateChecker::builder($domain.to_owned(), 25);
        c.set_starttls(CertificateCheckerStartTLSOptions::SMTP);
        c.set_expiry_threshold(Duration::from_secs(20*24*60*60)); // 20 days
        Box::new(c.build().unwrap())
      })
    };
  }
  macro_rules! http_and_tls {
    ($start_index:expr, $domain:expr, $expect_status:expr, $expect_contains:expr) => {
      http!($start_index, $domain, $expect_status, $expect_contains);
      tls!($start_index + 1, $domain);
    };
  }

  http_and_tls!(0<<4, "maowtm.org", 200, Some("Mao Wtm"));
  http_and_tls!(1<<4, "paper.sc", 200, Some("search engine for CIE papers"));
  http_and_tls!(2<<4, "static.maowtm.org", 404, Some("https://static.maowtm.org/svg/logo.svg"));
  http_and_tls!(3<<4, "death.maowtm.org", 200, Some("<!DOCTYPE HTML>"));
  http!(        4<<4, "oa.szlf.com", 302, None, Duration::from_secs(3));
  tls!(        (4<<4) + 1, "oa.szlf.com");
  http_and_tls!(5<<4, "status.maowtm.org", 200, Some("HTTP maowtm.org"));
  smtp!(        6<<4, "s1.maowtm.org");
  add_check!(   7<<4, "paper.sc query test", Duration::from_secs(30), {
    let mut c = HttpChecker::new("https://paper.sc/search/?as=json&query=test").unwrap();
    c.set_timeouts(Duration::from_secs(1), Duration::from_secs(5));
    c.expect_200();
    c.expect_response_contains(r#"{"response":"text","list""#);
    Box::new(c)
  });

  list
}
