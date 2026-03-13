use tonic::{Request, Status};

use crate::{
    app_error::AdminRpcError,
    db::{admin_repo::AdminRepo, cli_session_rows::CliSessionRow},
    state::AppState,
};
use sha2::{Digest, Sha256};

/// Extract bearer token from gRPC metadata Authorization header.
pub fn extract_bearer_token<T>(req: &Request<T>) -> Result<String, Status> {
    if let Some(v) = req.metadata().get("authorization") {
        if let Ok(s) = v.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ").map(str::trim) {
                if !token.is_empty() {
                    return Ok(token.to_string());
                }
            }
        }
    }
    Err(Status::unauthenticated("missing bearer token"))
}

pub async fn validate_session(state: &AppState, token: &str) -> Result<CliSessionRow, Status> {
    let repo = AdminRepo::new(state.db.clone());
    let session = repo
        .find_session_by_token(token)
        .await
        .map_err(internal_status)?
        .ok_or_else(|| Status::unauthenticated("invalid session token"))?;

    if session.revoked_at.is_some() {
        return Err(Status::unauthenticated("session revoked"));
    }
    if session.expires_at <= chrono::Utc::now() {
        return Err(Status::unauthenticated("session expired"));
    }
    let user = repo
        .find_user_by_id(session.user_id)
        .await
        .map_err(internal_status)?
        .ok_or_else(|| Status::unauthenticated("session user missing"))?;
    if !user.is_active {
        return Err(Status::unauthenticated("session user is disabled"));
    }

    Ok(session)
}

pub async fn login(
    state: &AppState,
    username: &str,
    password: &str,
) -> Result<(String, String), AdminRpcError> {
    let repo = AdminRepo::new(state.db.clone());
    let user = repo
        .find_user_by_username(username)
        .await?
        .ok_or(AdminRpcError::InvalidCredential)?;

    if !user.is_active {
        return Err(AdminRpcError::UserDisabled);
    }
    if !verify_password(
        &user.password_hash,
        password,
        &state.config.admin_password_pepper,
    ) {
        return Err(AdminRpcError::InvalidCredential);
    }

    let now = chrono::Utc::now();
    let expire_at = now + chrono::Duration::hours(12);
    let token = uuid::Uuid::new_v4().to_string();

    let session = CliSessionRow {
        session_id: 0,
        user_id: user.user_id,
        session_token: token.clone(),
        issued_at: now,
        expires_at: expire_at,
        revoked_at: None,
    };
    repo.insert_session(&session).await?;

    Ok((token, expire_at.to_rfc3339()))
}

fn internal_status(err: impl std::fmt::Display) -> Status {
    Status::internal(err.to_string())
}

fn verify_password(stored_hash: &str, password: &str, pepper: &str) -> bool {
    let mut parts = stored_hash.split('$');
    let Some(scheme) = parts.next() else {
        return false;
    };
    match scheme {
        "sha256" => {
            let Some(salt) = parts.next() else {
                return false;
            };
            let Some(expected) = parts.next() else {
                return false;
            };
            if parts.next().is_some() {
                return false;
            }
            let actual = password_digest_hex(salt, password, pepper);
            constant_time_eq(actual.as_bytes(), expected.as_bytes())
        }
        "sha256i" => {
            let Some(rounds_raw) = parts.next() else {
                return false;
            };
            let Ok(rounds) = rounds_raw.parse::<u32>() else {
                return false;
            };
            let Some(salt) = parts.next() else {
                return false;
            };
            let Some(expected) = parts.next() else {
                return false;
            };
            if parts.next().is_some() {
                return false;
            }
            if rounds == 0 {
                return false;
            }
            let actual = password_digest_hex_iter(salt, password, pepper, rounds);
            constant_time_eq(actual.as_bytes(), expected.as_bytes())
        }
        _ => false,
    }
}

fn password_digest_hex(salt: &str, password: &str, pepper: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    hasher.update(b":");
    hasher.update(pepper.as_bytes());
    hex::encode(hasher.finalize())
}

fn password_digest_hex_iter(salt: &str, password: &str, pepper: &str, rounds: u32) -> String {
    let mut current = password_digest_hex(salt, password, pepper).into_bytes();
    for _ in 1..rounds {
        let mut hasher = Sha256::new();
        hasher.update(salt.as_bytes());
        hasher.update(b":");
        hasher.update(&current);
        hasher.update(b":");
        hasher.update(pepper.as_bytes());
        current = hex::encode(hasher.finalize()).into_bytes();
    }
    String::from_utf8(current).unwrap_or_default()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0u8;
    for (&l, &r) in left.iter().zip(right.iter()) {
        diff |= l ^ r;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[test]
    fn extract_bearer_token_accepts_valid_header() {
        let mut req = Request::new(());
        req.metadata_mut()
            .insert("authorization", "Bearer test-token".parse().unwrap());

        let token = extract_bearer_token(&req).unwrap();

        assert_eq!(token, "test-token");
    }

    #[test]
    fn extract_bearer_token_rejects_missing_header() {
        let req = Request::new(());

        let err = extract_bearer_token(&req).unwrap_err();

        assert_eq!(err.code(), tonic::Code::Unauthenticated);
    }

    #[test]
    fn verify_password_accepts_sha256_format() {
        let stored = format!(
            "sha256$salt123${}",
            password_digest_hex("salt123", "secret", "pepper"),
        );

        assert!(verify_password(&stored, "secret", "pepper"));
        assert!(!verify_password(&stored, "wrong", "pepper"));
    }

    #[test]
    fn verify_password_rejects_unsupported_format() {
        assert!(!verify_password("plain$secret", "secret", ""));
    }

    #[test]
    fn verify_password_accepts_iterated_sha256_format() {
        let rounds = 20;
        let expected = password_digest_hex_iter("salt123", "secret", "pepper", rounds);
        let stored = format!("sha256i${rounds}$salt123${expected}");

        assert!(verify_password(&stored, "secret", "pepper"));
        assert!(!verify_password(&stored, "wrong", "pepper"));
    }
}
