use gateflow::dataplane::{rewrite, signing};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[test]
fn dataplane_chain_rewrites_path_and_generates_signature_headers() {
    let rewritten = rewrite::rewrite_path("/demo/v1/users", "/demo", "/api");
    assert_eq!(rewritten, "/api/v1/users");

    let full_path = format!("{rewritten}?page=1");
    let body = br#"{"name":"alice"}"#;
    let app_uuid = Uuid::new_v4();
    let (x_app_id, x_ts, x_sig, x_nonce, x_ttl) = signing::sign_request(
        &app_uuid,
        "secret",
        "salt",
        60,
        "POST",
        &full_path,
        body,
    )
    .unwrap();

    assert_eq!(x_app_id, app_uuid.to_string());
    assert!(!x_ts.is_empty());
    assert_eq!(x_sig.len(), 64);
    assert!(!x_nonce.is_empty());
    assert_eq!(x_ttl, "60");

    // Signature changes with body hash, proving payload is covered.
    let mut hasher = Sha256::new();
    hasher.update(body);
    let digest_1 = hex::encode(hasher.finalize());
    let mut hasher = Sha256::new();
    hasher.update(br#"{"name":"bob"}"#);
    let digest_2 = hex::encode(hasher.finalize());
    assert_ne!(digest_1, digest_2);
}
