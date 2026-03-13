use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::app_error::DataplaneError;

/// Compute gateway HMAC headers for downstream request.
pub fn sign_request(
    app_uuid: &uuid::Uuid,
    app_secret: &str,
    global_salt: &str,
    replay_window_secs: u64,
    method: &str,
    path_and_query: &str,
    body: &[u8],
) -> Result<(String, String, String, String, String), DataplaneError> {
    type HmacSha256 = Hmac<Sha256>;

    let ts = Utc::now().timestamp();
    let nonce = uuid::Uuid::new_v4().to_string();
    let ttl = replay_window_secs.to_string();
    let body_hash = {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(body);
        let digest = hasher.finalize();
        hex::encode(digest)
    };

    let to_sign = format!("{method}\n{path_and_query}\n{body_hash}\n{ts}\n{nonce}\n{ttl}");
    let key = format!("{global_salt}:{app_secret}");

    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|e| DataplaneError::Internal(format!("signing key error: {e}")))?;
    mac.update(to_sign.as_bytes());
    let sig = mac.finalize().into_bytes();
    let sig_hex = hex::encode(sig);

    Ok((app_uuid.to_string(), ts.to_string(), sig_hex, nonce, ttl))
}
