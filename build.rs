use std::env;
use std::process;
use std::path;

fn main () {
  if env::var("CARGO_FEATURE_WEB").is_ok() {
    let out_dir = path::Path::new(&env::var_os("OUT_DIR").unwrap()).to_owned();
    let web_source_path = path::Path::new(&env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("web/");
    let sass_path = web_source_path.join("assets/style.sass");
    if !process::Command::new("sassc")
      .current_dir(&out_dir)
      .args(&["--style", "compressed", "--sass", sass_path.to_str().unwrap(), out_dir.join("style.css").to_str().unwrap()])
      .status().unwrap().success() {
        eprintln!("Sass failed.");
        process::exit(1);
      }
  }
}
