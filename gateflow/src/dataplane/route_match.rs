use crate::{app_error::DataplaneError, db::AppRow, registry::store::AppRegistry};

/// Match incoming path to the best-fit app by mount_path (longest prefix wins).
pub fn match_app(path: &str, registry: &AppRegistry) -> Result<AppRow, DataplaneError> {
    let mut best: Option<(&String, &AppRow)> = None;

    for (mount, app) in registry.iter_mounts() {
        if !path.starts_with(mount) {
            continue;
        }
        // ensure boundary: either exact match or next char is '/'
        if path.len() > mount.len() && !path[mount.len()..].starts_with('/') {
            continue;
        }
        match &best {
            None => best = Some((mount, app)),
            Some((best_mount, _)) => {
                if mount.len() > best_mount.len() {
                    best = Some((mount, app));
                }
            }
        }
    }

    match best {
        Some((_, app)) => Ok(app.clone()),
        None => Err(DataplaneError::AppNotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::store::SearchType;
    use chrono::Utc;
    use uuid::Uuid;

    fn app(name: &str, mount_path: &str) -> AppRow {
        AppRow {
            app_uuid: Uuid::new_v4(),
            name: name.to_string(),
            target_url: format!("http://{name}.internal"),
            status: "Active".into(),
            mount_path: mount_path.to_string(),
            upstream_path: "/".into(),
            app_secret: "secret".into(),
            rate_limit_rps: None,
            allowed_source_ips: Vec::new(),
            blocked_source_ips: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn match_app_prefers_longest_mount_prefix() {
        let root = app("root", "/app");
        let nested = app("nested", "/app/admin");
        let mut registry = AppRegistry::new();
        registry.refresh(vec![root, nested.clone()]).unwrap();

        let matched = match_app("/app/admin/users", &registry).unwrap();

        assert_eq!(matched.app_uuid, nested.app_uuid);
    }

    #[test]
    fn match_app_enforces_path_boundary() {
        let app = app("demo", "/app");
        let mut registry = AppRegistry::new();
        registry.refresh(vec![app]).unwrap();

        let err = match_app("/application", &registry).unwrap_err();

        assert!(matches!(err, DataplaneError::AppNotFound));
        assert!(registry.get_app(SearchType::Mount("/app".into())).is_ok());
    }
}
