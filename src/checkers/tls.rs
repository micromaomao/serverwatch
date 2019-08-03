//! Check that a TLS server's certificate is valid and is not too close to expiry.

use crate::checkers::{Checker, CheckResult, CheckResultType};
use std::time;
use std::net;
use openssl;

/// Builder for [`CertificateChecker`](crate::checkers::tls::CertificateChecker).
/// Returned by
/// [`CertificateChecker::builder`](crate::checkers::tls::CertificateChecker::builder).
#[derive(Clone)]
pub struct CertificateCheckerBuilder {
  host: String,
  port: u16,
  failure_mode: CheckResultType,
  exipry_threshold: time::Duration,
  roots: CertificateCheckerRootOptions,
  fake_now: Option<time::SystemTime>,
}

#[derive(Clone)]
pub enum CertificateCheckerRootOptions {
  OpensslDefault,
  TrustThese(Vec<openssl::x509::X509>),
}

impl CertificateCheckerBuilder {
  /// Build the [`CertificateChecker`](crate::checkers::tls::CertificateChecker).
  pub fn build(self) -> Result<CertificateChecker, String> {
    let mut connector = openssl::ssl::SslConnector::builder(openssl::ssl::SslMethod::tls()).map_err(|e| format!("Setting up connector: {}", &e))?;
    if let CertificateCheckerRootOptions::TrustThese(roots) = self.roots {
      use openssl::x509::store;
      let mut st = store::X509StoreBuilder::new().map_err(|e| format!("Creating X509Store: {}", &e))?;
      for cert in roots.into_iter() {
        st.add_cert(cert).map_err(|e| format!("Adding cert to X509Store: {}", &e))?;
      }
      connector.set_verify_cert_store(st.build()).map_err(|e| format!("Connector::Set verify cert store: {}", &e))?;
    }
    Ok(CertificateChecker{
      host: self.host,
      port: self.port,
      failure_mode: self.failure_mode,
      exipry_threshold: self.exipry_threshold,
      openssl_connector: connector.build(),
      fake_now: self.fake_now,
    })
  }

  /// Set the [`CheckResultType`](crate::checkers::CheckResultType) returned when
  /// the certificate is about to expire, as determined by `exipry_threshold`.
  ///
  /// Defaults to `WARN`.
  ///
  /// Other errors, such as unable to connect to the server, returns `ERROR`
  /// regardless of this setting.
  pub fn set_failure_mode(&mut self, value: CheckResultType) {
    self.failure_mode = value;
  }

  /// Set the "threshold" of the expiry check.
  ///
  /// If the server certificate will expire within the duration, as of the time
  /// of the check (or
  /// [`fake_time`](crate::checkers::tls::CertificateCheckerBuilder::fake_time)),
  /// the check will fail with self.failure_mode, which defaults to `WARN`.
  ///
  /// Defaults to 2 days. Increase this to make the check stricter.
  pub fn set_expiry_threshold(&mut self, value: time::Duration) {
    self.exipry_threshold = value;
  }

  /// For testing. This make `check` act as if the system time is `value`.
  pub fn fake_time(&mut self, value: time::SystemTime) {
    self.fake_now = Some(value);
  }

  /// By default, the checker accepts all certificate issued by openssl's default
  /// trusted CAs. This change it so that only those in `value` are trusted.
  #[allow(non_snake_case)]
  pub fn set_trusted_CAs(&mut self, value: Vec<openssl::x509::X509>) {
    self.roots = CertificateCheckerRootOptions::TrustThese(value);
  }
}

/// Check that a TLS server's certificate is valid and is not too close to expiry.
///
/// ## Example
/// ```rust
/// # use serverwatch::checkers::{tls::CertificateChecker, Checker};
/// let mut checker = CertificateChecker::builder("google.com".to_owned(), 443).build().unwrap();
/// checker.check().expect();
/// ```
pub struct CertificateChecker {
  host: String,
  port: u16,
  failure_mode: CheckResultType,
  exipry_threshold: time::Duration,
  openssl_connector: openssl::ssl::SslConnector,
  fake_now: Option<time::SystemTime>,
}

impl CertificateChecker {
  /// Constructs a new `CertificateCheckerBuilder` and set the server to check to
  /// be `host` and port to be `port`.
  ///
  /// Name resolution is only performed when `check()` is called.
  pub fn builder(host: String, port: u16) -> CertificateCheckerBuilder {
    CertificateCheckerBuilder{
      host, port,
      failure_mode: CheckResultType::WARN,
      exipry_threshold: time::Duration::from_secs(2*24*60*60),
      roots: CertificateCheckerRootOptions::OpensslDefault,
      fake_now: None,
    }
  }
}

extern "C" {
  fn ASN1_TIME_cmp_time_t(s: *const openssl_sys::ASN1_TIME, t: libc::time_t) -> std::os::raw::c_int;
  fn X509_VERIFY_PARAM_set_time(param: *mut openssl_sys::X509_VERIFY_PARAM, t: libc::time_t);
  fn ASN1_TIME_set(s: *mut openssl_sys::ASN1_TIME, t: libc::time_t) -> *mut openssl_sys::ASN1_TIME;
  fn ASN1_TIME_diff(pday: *mut std::os::raw::c_int, psec: *mut std::os::raw::c_int, from: *const openssl_sys::ASN1_TIME, to: *const openssl_sys::ASN1_TIME) -> std::os::raw::c_int;
}

impl Checker for CertificateChecker {
  fn check(&mut self) -> CheckResult {
    use std::ops::Add;
    use foreign_types::{ForeignType, ForeignTypeRef};
    let now = self.fake_now.unwrap_or(time::SystemTime::now());
    let now_time_t = if now > time::UNIX_EPOCH {
      now.duration_since(time::UNIX_EPOCH).unwrap().as_secs() as time_t
    } else {
      -(time::UNIX_EPOCH.duration_since(now).unwrap().as_secs() as time_t)
    };
    let compare_with = now.add(self.exipry_threshold);
    use libc::time_t;
    let compare_with: time_t = if compare_with > time::UNIX_EPOCH {
      compare_with.duration_since(time::UNIX_EPOCH).unwrap().as_secs() as time_t
    } else {
      -(time::UNIX_EPOCH.duration_since(compare_with).unwrap().as_secs() as time_t)
    };
    let conn = match net::TcpStream::connect((&self.host[..], self.port)) {
      Ok(k) => k,
      Err(e) => return CheckResult::error(Some(format!("Unable to connect: {}", &e)))
    };
    let mut ssl = match self.openssl_connector.configure() {
      Ok(k) => k,
      Err(e) => return CheckResult::error(Some(format!("Allocating SSL: {}", &e)))
    };
    unsafe { X509_VERIFY_PARAM_set_time(ssl.param_mut().as_ptr(), now_time_t) };
    let mut tls_stream = match ssl.connect(&self.host, conn) {
      Ok(k) => k,
      Err(e) => return CheckResult::error(Some(format!("OpenSSL handshake: {}", &e)))
    };
    let peer_cert = match tls_stream.ssl().peer_certificate() {
      Some(c) => c,
      None => return CheckResult::error(Some(format!("No peer certificate?")))
    };
    let not_after = peer_cert.not_after();
    let ret_ok = unsafe { ASN1_TIME_cmp_time_t(not_after.as_ptr(), compare_with) } >= 0;
    std::thread::spawn(move || {
      if {let s = tls_stream.shutdown(); s.is_ok() && s.unwrap() == openssl::ssl::ShutdownResult::Sent} {
        let _ = tls_stream.shutdown();
      }
    });
    if ret_ok {
      return CheckResult::up(Some(format!("Certificate valid until {}", &not_after.to_string())));
    } else {
      let now_asn1 = unsafe { openssl::asn1::Asn1Time::from_ptr(ASN1_TIME_set(std::ptr::null_mut(), now_time_t)) };
      let mut diff_day: std::os::raw::c_int = 0;
      let mut diff_sec: std::os::raw::c_int = 0;
      unsafe { ASN1_TIME_diff(&mut diff_day as *mut _, &mut diff_sec as *mut _, now_asn1.as_ptr(), not_after.as_ptr()) };
      let mut valid_rem_days: f32 = diff_day as f32;
      valid_rem_days += diff_sec as f32 / (24*60*60) as f32;
      return CheckResult{
        result_type: self.failure_mode,
        info: Some(format!("Certificate expiring in {:.1} days: Certificate valid until {}; current time is {}.", valid_rem_days, &not_after.to_string(), &now_asn1.to_string())),
      };
    }
  }
}

#[test]
fn cert_checker_test() {
  let mut chk = CertificateChecker::builder("expired.badssl.com".to_owned(), 443);
  // Expires on: 12 April 2015, 23:59:59 GMT
  let not_after = time::SystemTime::UNIX_EPOCH + time::Duration::from_secs(1428883199);
  let one_hour = time::Duration::from_secs(60*60);
  chk.set_expiry_threshold(2*one_hour);

  // Good
  chk.fake_time(not_after - 3*one_hour);
  chk.clone().build().unwrap().check().expect();

  // Not yet valid
  chk.fake_time(not_after - 365*24*one_hour);
  chk.clone().build().unwrap().check().expect_err_contains("certificate is not yet valid");

  // Expired
  chk.fake_time(not_after + 365*24*one_hour);
  chk.clone().build().unwrap().check().expect_err_contains("certificate has expire");

  // About to expire
  chk.fake_time(not_after - 1*one_hour);
  assert_eq!(chk.clone().build().unwrap().check().result_type, CheckResultType::WARN);

  // Test for adding CA
  let mut chk = CertificateChecker::builder("superfish.badssl.com".to_owned(), 443);
  chk.clone().build().unwrap().check().expect_err();
  chk.set_trusted_CAs(vec![openssl::x509::X509::from_der(include_bytes!("./tls_test_badssl_superfish.der")).unwrap()]);
  chk.build().unwrap().check().expect();
}
