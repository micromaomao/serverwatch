use openssl::ec;
use std::time;
fn main() {
  let b64url = base64::Config::new(base64::CharacterSet::UrlSafe, false);
  // server_key: &ec::EcKey<openssl::pkey::Private>, endpoint: &str, client_key_raw: &[u8], auth_secret: &[u8], push_body: &[u8], ttl: time::Duration
  let server_key = openssl::ec::EcKey::private_key_from_pem(include_bytes!("keys/app_server.key")).unwrap();
  let server_key = &server_key;
  let endpoint = "https://push.example.net/push/JzLQ3raZJfFBR0aqvOMsLrt54w4rJUsV";
  let client_key_raw = base64::decode_config("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4", b64url).unwrap();
  let client_key_raw = &client_key_raw;
  let auth_secret = base64::decode_config("BTBZMqHH6r4Tts7J_aSIgg", b64url).unwrap();
  let auth_secret = &auth_secret;
  let push_body = "When I grow up, I want to be a watermelon".as_bytes();
  let ttl = time::Duration::from_secs(10);

  let endpoint_url = reqwest::Url::parse(endpoint).unwrap();

  let mut bnctx = openssl::bn::BigNumContext::new().unwrap();
  let p256curve = ec::EcGroup::from_curve_name(openssl::nid::Nid::X9_62_PRIME256V1).unwrap();

  let client_key_point = ec::EcPoint::from_bytes(&*p256curve, client_key_raw, &mut bnctx).unwrap();
  let client_key = ec::EcKey::from_public_key(&p256curve, &*client_key_point).unwrap();
  client_key.check_key().unwrap();

  let auth_header = get_vapid_auth_header(server_key, &endpoint_url.origin().unicode_serialization()).unwrap();

  use openssl::derive;
  // let our_ec_key = ec::EcKey::generate(&p256curve).unwrap();
  let our_ec_key = ec::EcKey::from_private_components(&*p256curve, &*openssl::bn::BigNum::from_slice(&base64::decode_config("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw", b64url).unwrap()).unwrap(), &*openssl::ec::EcPoint::from_bytes(&p256curve, &base64::decode_config("BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8", b64url).unwrap(), &mut bnctx).unwrap()).unwrap();
  let our_ec_key_pub = our_ec_key.public_key().to_bytes(our_ec_key.group(), openssl::ec::PointConversionForm::UNCOMPRESSED, &mut bnctx).unwrap();
  let _our_ec_pkey = openssl::pkey::PKey::from_ec_key(our_ec_key).unwrap();
  let mut dh = derive::Deriver::new(&_our_ec_pkey).unwrap();
  let _client_pkey = openssl::pkey::PKey::from_ec_key(client_key).unwrap();
  dh.set_peer(&_client_pkey).unwrap();
  let dh_secret = dh.derive_to_vec().unwrap();

  println!("dh_secret = {}", base64::encode_config(&dh_secret, b64url));

  let mut key_info = Vec::from("WebPush: info\0".as_bytes());
  key_info.extend_from_slice(client_key_raw);
  key_info.extend_from_slice(&our_ec_key_pub);

  println!("key_info = {}", base64::encode_config(&key_info, b64url));

  let mut ikm = [0u8; 32];
  openssl_hkdf(auth_secret, &dh_secret, &key_info, &mut ikm).unwrap();

  println!("ikm = {}", base64::encode_config(&ikm, b64url));

  // let mut salt = [0u8; 16];
  // openssl::rand::rand_bytes(&mut salt).unwrap();
  let salt = base64::decode_config("DGv6ra1nlYgDCS1FRnbzlw", b64url).unwrap();

  let mut cek = [0u8; 16];
  openssl_hkdf(&salt, &ikm, "Content-Encoding: aes128gcm\0".as_bytes(), &mut cek).unwrap();
  let mut iv = [0u8; 12];
  openssl_hkdf(&salt, &ikm, "Content-Encoding: nonce\0".as_bytes(), &mut iv).unwrap();

  println!("iv = {}", base64::encode_config(&iv, b64url));

  let mut plaintext = Vec::new();
  plaintext.extend_from_slice(push_body);
  plaintext.push(2u8); // Record padding

  let record_size = 4096u32;
  let mut header_block = Vec::new();
  header_block.extend_from_slice(&salt);
  header_block.extend_from_slice(&u32::to_be_bytes(record_size)); // last record can be smaller than record_size
  header_block.push(our_ec_key_pub.len() as u8);
  header_block.extend_from_slice(&our_ec_key_pub);

  // let record_body = openssl::symm::encrypt(openssl::symm::Cipher::aes_128_gcm(), &cek, Some(&iv), &plaintext).unwrap(); // This does not add auth tag
  let mut crypter = openssl::symm::Crypter::new(openssl::symm::Cipher::aes_128_gcm(), openssl::symm::Mode::Encrypt, &cek, Some(&iv)).unwrap();
  let mut ciphertext = vec![0u8; plaintext.len()];
  let mut written = crypter.update(&plaintext, &mut ciphertext).unwrap();
  written += crypter.finalize(&mut ciphertext[written..]).unwrap();
  assert_eq!(written, plaintext.len());

  let mut body_to_send = header_block;
  body_to_send.extend_from_slice(&ciphertext);
  let _off = body_to_send.len();
  body_to_send.resize(_off + 16, 0u8);
  crypter.get_tag(&mut body_to_send[_off..]).unwrap();

  println!("body = {}", base64::encode_config(&body_to_send, b64url));

  // let mut res = http_client.post(endpoint_url)
  //   .header("Content-Encoding", "aes128gcm")
  //   .header("Content-Type", "application/octet-stream")
  //   .header("Authorization", auth_header)
  //   .header("TTL", format!("{}", ttl.as_secs()))
  //   .body(record_body)
  //   .send().map_err(|e| format!("while sending push request: {}", e))?;

  // let status_code = res.status().as_u16();
  // if status_code == 200 || status_code == 201 {
  //   Ok(())
  // } else {
  //   Err(format!("Push endpoint responsed with {}: {}", status_code, res.text().unwrap_or("Invalid UTF8".to_owned())))
  // }
}


fn get_signed_jwt(server_key: &ec::EcKey<openssl::pkey::Private>, aud: &str) -> Result<String, String> {
  let jwt_header = r#"{"alg":"ES256"}"#;
  let exp = (time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap() + time::Duration::from_secs(12*60*60)).as_secs();
  // TODO: don't hard-code these
  let jwt_body = format!(r#"{{"aud": "{}", "exp": "{}", "sub": "mailto:push-jwt@maowtm.org"}}"#, aud, exp);
  let b64urlcfg = base64::Config::new(base64::CharacterSet::UrlSafe, false);
  let signing_input = format!("{}.{}", base64::encode_config(jwt_header, b64urlcfg.clone()), base64::encode_config(&jwt_body, b64urlcfg.clone()));
  let hashed_signing_input = openssl::sha::sha256(signing_input.as_bytes());
  let sig = openssl::ecdsa::EcdsaSig::sign(&hashed_signing_input, server_key).map_err(|e| format!("{}", &e))?;
  let mut sig_bytes = sig.r().to_vec();
  sig_bytes.extend_from_slice(&sig.s().to_vec());
  let final_jwt = format!("{}.{}", signing_input, base64::encode_config(&sig_bytes, b64urlcfg));
  Ok(final_jwt)
}

fn get_vapid_auth_header(server_key: &ec::EcKey<openssl::pkey::Private>, aud: &str) -> Result<String, String> {
  let t = get_signed_jwt(server_key, aud)?;
  let k = server_key.public_key().to_bytes(server_key.group(), openssl::ec::PointConversionForm::UNCOMPRESSED, &mut *openssl::bn::BigNumContext::new().map_err(|e| format!("{}", e))?).map_err(|e| format!("{}", e))?;
  Ok(format!("vapid t={},k={}", t, base64::encode_config(&k, base64::Config::new(base64::CharacterSet::UrlSafe, false))))
}

fn openssl_hkdf(salt: &[u8], ikm: &[u8], info: &[u8], buf: &mut [u8]) -> Result<(), openssl::error::ErrorStack> {
  let prk = hmac(salt, ikm)?;
  let mut last = Vec::new();
  let mut i = 0usize;
  let mut out_off = 0usize;
  while out_off < buf.len() {
    let mut data = last.clone();
    data.extend_from_slice(info);
    data.push((i + 1) as u8);
    last = hmac(&prk, &data)?;
    let push_len = usize::min(buf.len() - out_off, last.len());
    buf[out_off..out_off + push_len].clone_from_slice(&last[0..push_len]);
    out_off += push_len;
    i += 1;
  }
  Ok(())
}

fn hmac(key: &[u8], data: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
  let _pkey_hmac = openssl::pkey::PKey::hmac(key)?;
  let mut signer = openssl::sign::Signer::new(openssl::hash::MessageDigest::sha256(), &_pkey_hmac)?;
  signer.update(data)?;
  signer.sign_to_vec()
}
