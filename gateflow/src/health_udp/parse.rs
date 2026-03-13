use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::app_error::HealthUdpError;
use crate::domain::health::HealthReport;

#[derive(Debug, Deserialize)]
struct IncomingHealth {
    app_uuid: Uuid,
    name: String,
    checked_at: i64,
    ok: bool,
    #[serde(default)]
    status_code: u16,
    #[serde(default)]
    latency_ms: u32,
}

#[derive(Debug, Deserialize)]
struct IncomingBatch {
    reports: Vec<IncomingHealth>,
}

pub fn parse_datagram(buf: &[u8]) -> Result<Vec<HealthReport>, HealthUdpError> {
    let batch = serde_json::from_slice::<IncomingBatch>(buf)
        .map_err(|e| HealthUdpError::Message(format!("invalid health batch json: {e}")))?;

    let mut out = Vec::with_capacity(batch.reports.len());
    for item in batch.reports {
        out.push(to_report(item)?);
    }
    Ok(out)
}

fn to_report(incoming: IncomingHealth) -> Result<HealthReport, HealthUdpError> {
    let checked_at = parse_checked_at(incoming.checked_at)?;
    Ok(HealthReport {
        app_uuid: incoming.app_uuid,
        name: incoming.name,
        checked_at,
        ok: incoming.ok,
        status_code: incoming.status_code,
        latency_ms: incoming.latency_ms,
    })
}

fn parse_checked_at(raw: i64) -> Result<DateTime<Utc>, HealthUdpError> {
    // healthd sends Unix milliseconds; keep backward compatibility with seconds.
    if raw.abs() >= 1_000_000_000_000 {
        DateTime::<Utc>::from_timestamp_millis(raw)
            .ok_or_else(|| HealthUdpError::Message("invalid timestamp".into()))
    } else {
        DateTime::<Utc>::from_timestamp(raw, 0)
            .ok_or_else(|| HealthUdpError::Message("invalid timestamp".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_datagram_reads_valid_batch_payload() {
        let payload = br#"{
            "tick_id":1,
            "checked_at":1735689600000,
            "reports":[
                {
                    "app_uuid":"550e8400-e29b-41d4-a716-446655440000",
                    "name":"demo",
                    "checked_at":1735689600000,
                    "ok":true,
                    "status_code":200,
                    "latency_ms":12
                }
            ]
        }"#;

        let report = parse_datagram(payload).unwrap();

        assert_eq!(report.len(), 1);
        assert_eq!(report[0].name, "demo");
        assert!(report[0].ok);
        assert_eq!(report[0].status_code, 200);
        assert_eq!(report[0].latency_ms, 12);
    }

    #[test]
    fn parse_datagram_rejects_single_payload() {
        let single = br#"{
            "app_uuid":"550e8400-e29b-41d4-a716-446655440000",
            "name":"demo",
            "checked_at":1735689600,
            "ok":true,
            "status_code":200,
            "latency_ms":12
        }"#;
        assert!(parse_datagram(single).is_err());
    }

    #[test]
    fn parse_datagram_rejects_bad_json() {
        assert!(parse_datagram(br#"{"bad":true}"#).is_err());
    }

    #[test]
    fn parse_datagram_accepts_missing_status_code_and_latency() {
        let payload = br#"{
            "reports":[
                {
                    "app_uuid":"550e8400-e29b-41d4-a716-446655440000",
                    "name":"demo",
                    "checked_at":1735689600000,
                    "ok":false
                }
            ]
        }"#;

        let report = parse_datagram(payload).unwrap();

        assert_eq!(report.len(), 1);
        assert_eq!(report[0].status_code, 0);
        assert_eq!(report[0].latency_ms, 0);
    }
}
