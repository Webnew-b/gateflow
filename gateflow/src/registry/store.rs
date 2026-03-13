use std::collections::HashMap;

use crate::{app_error::AppError, db::AppRow};
use uuid::Uuid;

#[derive(Clone, Default)]
struct AppList<K>(HashMap<K, AppRow>);

#[derive(Clone, Default)]
pub struct AppRegistry {
    /// 根据 UUID 查
    apps_by_uuid: AppList<Uuid>,
    /// 根据 name 查
    apps_by_name: AppList<String>,
    /// 根据 mount_path 查
    apps_by_mount: AppList<String>,
}

impl<K: std::cmp::Eq + std::hash::Hash> AppList<K> {
    pub fn get(&self, key: &K) -> Option<&AppRow> {
        self.0.get(key)
    }
}

pub enum SearchType {
    Uuid(Uuid),
    Name(String),
    Mount(String),
}

impl AppRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 用数据库查询结果一次性刷新三个索引。
    pub fn refresh(&mut self, apps: Vec<AppRow>) -> Result<(), AppError> {
        let mut by_uuid = HashMap::new();
        let mut by_name = HashMap::new();
        let mut by_mount = HashMap::new();

        for app in apps.into_iter() {
            by_uuid.insert(app.app_uuid, app.clone());
            by_name.insert(app.name.clone(), app.clone());
            by_mount.insert(app.mount_path.clone(), app.clone());
        }

        self.apps_by_uuid = AppList(by_uuid);
        self.apps_by_name = AppList(by_name);
        self.apps_by_mount = AppList(by_mount);

        Ok(())
    }

    /// 遍历 mount_path → AppRow 的映射，供数据面前缀匹配使用。
    pub fn iter_mounts(&self) -> impl Iterator<Item = (&String, &AppRow)> {
        self.apps_by_mount.0.iter()
    }

    pub fn get_app(&self, search_param: SearchType) -> Result<AppRow, AppError> {
        let res = match search_param {
            SearchType::Uuid(app_uuid) => self.apps_by_uuid.get(&app_uuid),
            SearchType::Name(n) => self.apps_by_name.get(&n),
            SearchType::Mount(m) => self.apps_by_mount.get(&m),
        }
        .ok_or(AppError::Registry(
            crate::app_error::RegistryError::AppNotFound,
        ))?
        .clone();
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn refresh_populates_name_and_mount_indexes() {
        let mut registry = AppRegistry::new();
        let demo = app("demo", "/demo");

        registry.refresh(vec![demo.clone()]).unwrap();

        assert_eq!(
            registry
                .get_app(SearchType::Name("demo".into()))
                .unwrap()
                .app_uuid,
            demo.app_uuid
        );
        assert_eq!(
            registry
                .get_app(SearchType::Mount("/demo".into()))
                .unwrap()
                .app_uuid,
            demo.app_uuid
        );
        assert_eq!(
            registry
                .get_app(SearchType::Uuid(demo.app_uuid))
                .unwrap()
                .app_uuid,
            demo.app_uuid
        );
    }

}
