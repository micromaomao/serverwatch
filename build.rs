use std::env;
use std::process;
use std::path;

fn main () {
  if env::var("CARGO_FEATURE_WEB").is_ok() {
    let web_assets_path = path::Path::new(&env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("web/assets");
    // TODO
  }
}
