use crate::models::{FilterConfig, Mode, RecommendedSite, UrlRule};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct FilterEngine {
    config: Arc<RwLock<FilterConfig>>,
    pin_hash: Arc<RwLock<Option<String>>>,
    category_cache: Arc<RwLock<HashMap<String, Option<String>>>>,
    config_path: Option<PathBuf>,
}

impl FilterEngine {
    #[allow(dead_code)]
    pub fn new(config: FilterConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            pin_hash: Arc::new(RwLock::new(None)),
            category_cache: Arc::new(RwLock::new(HashMap::new())),
            config_path: None,
        }
    }

    pub fn with_config_path(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    pub fn get_config(&self) -> Arc<RwLock<FilterConfig>> {
        self.config.clone()
    }

    pub fn get_pin_hash(&self) -> Arc<RwLock<Option<String>>> {
        self.pin_hash.clone()
    }

    pub fn update_config(&self, config: FilterConfig) {
        if let Ok(mut cfg) = self.config.write() {
            *cfg = config;
        }
        self.save_to_disk();
    }

    pub fn save_to_disk(&self) {
        if let Some(path) = &self.config_path {
            if let Ok(config) = self.config.read() {
                if let Ok(json) = serde_json::to_string_pretty(&*config) {
                    if let Some(parent) = path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if let Err(e) = fs::write(path, json) {
                        log::error!("Failed to save config: {}", e);
                    } else {
                        log::info!("💾 配置已保存到磁盘");
                    }
                }
            }
        }
    }

    pub fn load_from_disk(&mut self) {
        if let Some(path) = &self.config_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(loaded_config) = serde_json::from_str::<FilterConfig>(&content) {
                        if let Ok(mut config) = self.config.write() {
                            *config = loaded_config;
                            log::info!("📂 配置已从磁盘加载: 规则数量={}", config.url_rules.len());
                        }
                    }
                }
            }
        }
    }

    pub fn is_parent_mode(&self) -> bool {
        self.config
            .read()
            .map(|c| c.mode == Mode::Parent)
            .unwrap_or(false)
    }

    pub fn check_access(&self, domain: &str, url: &str) -> (bool, Option<String>) {
        let config = match self.config.read() {
            Ok(c) => c,
            Err(_) => return (true, None),
        };

        if config.mode == Mode::Parent {
            return (true, None);
        }

        let (allowed, reason) = config.time_rule.is_access_allowed();
        if !allowed {
            log::warn!("🚫 [Block] 域名: {} | 原因: {}", domain, reason);
            return (false, Some(reason));
        }

        let (url_allowed, url_reason) = config.check_url(url, domain);
        if !url_allowed {
            log::warn!("🚫 [Block] 域名: {} | 规则: {:?}", domain, url_reason);
            return (false, url_reason);
        }

        if let Some(category) = self.lookup_category(domain) {
            if config.is_blocked_category(&category) {
                log::warn!("🚫 [Block] 域名: {} | 类别: {}", domain, category);
                return (false, Some(format!("该类别({})已被禁止访问", category)));
            }
        }

        (true, None)
    }

    pub fn check_sni(&self, sni: &str) -> (bool, Option<String>) {
        self.check_access(sni, sni)
    }

    pub fn check_dns(&self, domain: &str) -> (bool, Option<String>) {
        self.check_access(domain, domain)
    }

    fn lookup_category(&self, domain: &str) -> Option<String> {
        {
            let cache = self.category_cache.read().ok()?;
            if let Some(cat) = cache.get(domain) {
                return cat.clone();
            }
        }

        let category = self.query_category(domain);

        {
            let mut cache = self.category_cache.write().ok()?;
            cache.insert(domain.to_string(), category.clone());
        }

        category
    }

    fn query_category(&self, domain: &str) -> Option<String> {
        let config = self.config.read().ok()?;

        let known_blocked: HashMap<&str, &str> = [
            ("pornhub", "porn"),
            ("xvideos", "porn"),
            ("xhamster", "porn"),
            ("instagram", "social"),
            ("tiktok", "social"),
            ("twitter", "social"),
            ("facebook", "social"),
            ("youtube", "video"),
            ("netflix", "video"),
            ("bilibili", "video"),
            ("douyin", "video"),
            ("wetv", "video"),
            ("bet365", "gambling"),
            ("pokerstars", "gambling"),
            ("steam", "gaming"),
            ("epicgames", "gaming"),
        ]
        .iter()
        .cloned()
        .collect();

        for (blocked_domain, category) in &known_blocked {
            if domain.contains(blocked_domain) || blocked_domain.contains(domain) {
                return Some(category.to_string());
            }
        }

        if config.cloud_fallback {
            None
        } else {
            None
        }
    }

    pub fn add_url_rule(&self, rule: UrlRule) {
        let rule_name = rule.name.clone();
        if let Ok(mut config) = self.config.write() {
            config.url_rules.push(rule);
            log::info!(
                "➕ 规则已添加: {} (总数: {})",
                rule_name,
                config.url_rules.len()
            );
        }
        self.save_to_disk();
    }

    pub fn remove_url_rule(&self, rule_id: &str) {
        if let Ok(mut config) = self.config.write() {
            let before = config.url_rules.len();
            config.url_rules.retain(|r| r.id != rule_id);
            let after = config.url_rules.len();
            if before != after {
                log::info!("🗑️ 规则已删除 (总数: {} -> {})", before, after);
            }
        }
        self.save_to_disk();
    }

    pub fn get_recommended_sites(&self) -> Vec<RecommendedSite> {
        self.config
            .read()
            .map(|c| c.recommended_sites.clone())
            .unwrap_or_default()
    }

    pub fn add_recommended_site(&self, site: RecommendedSite) {
        if let Ok(mut config) = self.config.write() {
            config.recommended_sites.push(site);
        }
    }

    pub fn remove_recommended_site(&self, site_id: &str) {
        if let Ok(mut config) = self.config.write() {
            config.recommended_sites.retain(|s| s.id != site_id);
        }
    }

    pub fn update_recommended_site(&self, site: RecommendedSite) {
        if let Ok(mut config) = self.config.write() {
            if let Some(existing) = config
                .recommended_sites
                .iter_mut()
                .find(|s| s.id == site.id)
            {
                *existing = site;
            }
        }
    }

    pub fn set_mode(&self, mode: Mode) {
        if let Ok(mut config) = self.config.write() {
            config.mode = mode;
        }
    }

    pub fn get_state(&self) -> FilterEngineState {
        let config = self.config.read().unwrap_or_else(|e| e.into_inner());
        FilterEngineState {
            mode: config.mode,
            is_blocked: config.time_rule.remaining_minutes() == 0,
            remaining_minutes: config.time_rule.remaining_minutes(),
            used_minutes: config.time_rule.usage_today.used_minutes,
            max_minutes: config.time_rule.max_daily_minutes,
            active_rules: config.url_rules.len(),
        }
    }

    pub fn verify_pin(&self, pin: &str, hash: &str) -> bool {
        let computed_hash = hash_pin(pin);
        computed_hash == hash
    }

    pub fn set_pin_hash(&self, hash: String) {
        if let Ok(mut pin) = self.pin_hash.write() {
            *pin = Some(hash);
        }
    }

    pub fn has_pin(&self) -> bool {
        self.pin_hash.read().map(|p| p.is_some()).unwrap_or(false)
    }
}

pub fn hash_pin(pin: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    pin.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FilterEngineState {
    pub mode: Mode,
    pub is_blocked: bool,
    pub remaining_minutes: u32,
    pub used_minutes: u32,
    pub max_minutes: u32,
    pub active_rules: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parent_mode_always_allowed() {
        let engine = FilterEngine::new(FilterConfig {
            mode: Mode::Parent,
            ..Default::default()
        });

        let (allowed, _) = engine.check_access("pornhub.com", "https://pornhub.com");
        assert!(allowed);
    }

    #[test]
    fn test_blocked_domain() {
        let engine = FilterEngine::new(FilterConfig {
            mode: Mode::Teen,
            url_rules: vec![UrlRule {
                id: "1".to_string(),
                name: "Block porn".to_string(),
                pattern: "pornhub".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        });

        let (allowed, reason) = engine.check_access("pornhub.com", "https://pornhub.com");
        assert!(!allowed);
        assert!(reason.is_some());
    }

    #[test]
    fn test_wildcard_pattern() {
        let engine = FilterEngine::new(FilterConfig {
            mode: Mode::Teen,
            url_rules: vec![UrlRule {
                id: "1".to_string(),
                name: "Block all *.example.com".to_string(),
                pattern: "*.example.com".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        });

        let (allowed, _) = engine.check_access("sub.example.com", "https://sub.example.com");
        assert!(!allowed);
    }
}
