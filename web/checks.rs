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
    ($index:expr, $domain:expr) => {
      add_check!($index, concat!("HTTP ", $domain), Duration::from_secs(10), {
        let mut c = HttpChecker::new(concat!("https://", $domain, "/")).unwrap();
        c.set_timeouts(Duration::from_secs(1), Duration::from_secs(5));
        Box::new(c)
      });
    };
    ($index:expr, $domain:expr, $warn_timeout:expr) => {
      add_check!($index, concat!("HTTP ", $domain), Duration::from_secs(10), {
        let mut c = HttpChecker::new(concat!("https://", $domain, "/")).unwrap();
        c.set_timeouts($warn_timeout, Duration::from_secs(5));
        Box::new(c)
      });
    };
  }
  macro_rules! tls {
    ($index:expr, $domain:expr) => {
      add_check!($index, concat!("TLS ", $domain), Duration::from_secs(60), {
        let mut c = CertificateChecker::builder($domain.to_owned(), 443);
        c.set_expiry_threshold(Duration::from_secs(10*24*60*60)); // 10 days
        Box::new(c.build().unwrap())
      })
    };
  }
  macro_rules! smtp {
    ($index:expr, $domain:expr) => {
      add_check!($index, concat!("SMTP ", $domain), Duration::from_secs(60), {
        let mut c = CertificateChecker::builder($domain.to_owned(), 25);
        c.set_starttls(CertificateCheckerStartTLSOptions::SMTP);
        c.set_expiry_threshold(Duration::from_secs(10*24*60*60)); // 10 days
        Box::new(c.build().unwrap())
      })
    };
  }
  macro_rules! http_and_tls {
    ($start_index:expr, $domain:expr) => {
      http!($start_index, $domain);
      tls!($start_index + 1, $domain);
    };
  }
  http_and_tls!(0<<4, "maowtm.org");
  http_and_tls!(1<<4, "paper.sc");
  http_and_tls!(2<<4, "static.maowtm.org");
  http_and_tls!(3<<4, "localhost");
  smtp!(        4<<4, "gmail-smtp-in.l.google.com");
  list
}
