use gateflow::health_udp::{parse, store};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[tokio::test]
async fn udp_health_report_is_parsed_and_stored() {
    let payload = br#"{
        "tick_id": 1,
        "checked_at": 1735689600000,
        "reports": [
            {
                "app_uuid":"550e8400-e29b-41d4-a716-446655440000",
                "name":"demo",
                "checked_at":1735689600000,
                "ok":true,
                "status_code":200,
                "latency_ms":15
            }
        ]
    }"#;

    let reports = parse::parse_datagram(payload).unwrap();
    let store_ref = Arc::new(RwLock::new(HashMap::<Uuid, gateflow::domain::health::AppHealth>::new()));
    store::upsert_many(&store_ref, &reports).await;

    let guard = store_ref.read().await;
    let app = guard
        .get(&Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap())
        .unwrap();
    assert!(app.ok);
    assert_eq!(app.status_code, 200);
    assert_eq!(app.latency_ms, 15);
}
