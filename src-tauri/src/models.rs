use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Parent,
    Teen,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Parent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    pub start: String,
    pub end: String,
    pub days: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub date: String,
    pub used_minutes: u32,
}

impl Default for UsageRecord {
    fn default() -> Self {
        Self {
            date: String::new(),
            used_minutes: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRule {
    pub enabled: bool,
    pub slots: Vec<TimeSlot>,
    pub max_daily_minutes: u32,
    pub usage_today: UsageRecord,
}

impl Default for TimeRule {
    fn default() -> Self {
        Self {
            enabled: false,
            slots: vec![],
            max_daily_minutes: 480,
            usage_today: UsageRecord::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UrlRuleType {
    Block,
    Allow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlRule {
    pub id: String,
    pub name: String,
    pub rule_type: UrlRuleType,
    pub pattern: String,
    pub category: Option<String>,
    pub enabled: bool,
}

impl Default for UrlRule {
    fn default() -> Self {
        Self {
            id: uuid_v4(),
            name: String::new(),
            rule_type: UrlRuleType::Block,
            pattern: String::new(),
            category: None,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedSite {
    pub id: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub icon: String,
    pub category: String,
    pub enabled: bool,
}

impl Default for RecommendedSite {
    fn default() -> Self {
        Self {
            id: uuid_v4(),
            title: String::new(),
            description: String::new(),
            url: String::new(),
            icon: String::new(),
            category: String::new(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub mode: Mode,
    pub time_rule: TimeRule,
    pub url_rules: Vec<UrlRule>,
    pub recommended_sites: Vec<RecommendedSite>,
    pub block_categories: HashSet<String>,
    pub cloud_fallback: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Teen,
            time_rule: TimeRule::default(),
            url_rules: default_url_rules(),
            recommended_sites: default_recommended_sites(),
            block_categories: default_blocked_categories(),
            cloud_fallback: true,
        }
    }
}

fn default_url_rules() -> Vec<UrlRule> {
    vec![
        UrlRule {
            id: uuid_v4(),
            name: "示例阻止规则".to_string(),
            rule_type: UrlRuleType::Block,
            pattern: "example-block.com".to_string(),
            category: Some("测试".to_string()),
            enabled: true,
        },
        UrlRule {
            id: uuid_v4(),
            name: "赌博网站".to_string(),
            rule_type: UrlRuleType::Block,
            pattern: "bet365".to_string(),
            category: Some("赌博".to_string()),
            enabled: true,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub config: FilterConfig,
    pub pin_hash: Option<String>,
    pub is_blocked: bool,
    pub block_reason: Option<String>,
}

fn default_blocked_categories() -> HashSet<String> {
    let mut set = HashSet::new();
    set.insert("porn".to_string());
    set.insert("gambling".to_string());
    set.insert("violence".to_string());
    set.insert("drugs".to_string());
    set.insert("hate".to_string());
    set
}

pub fn default_recommended_sites() -> Vec<RecommendedSite> {
    vec![
        RecommendedSite {
            id: "1".to_string(),
            title: "国家地理".to_string(),
            description: "探索世界，增长见识".to_string(),
            url: "https://www.nationalgeographic.com.cn".to_string(),
            icon: "🌍".to_string(),
            category: "科普探索".to_string(),
            enabled: true,
        },
        RecommendedSite {
            id: "2".to_string(),
            title: "动物世界".to_string(),
            description: "了解神奇的动物王国".to_string(),
            url: "https://www.zoo.cn".to_string(),
            icon: "🦁".to_string(),
            category: "科普探索".to_string(),
            enabled: true,
        },
        RecommendedSite {
            id: "3".to_string(),
            title: "中国数字科技馆".to_string(),
            description: "趣味科普，启迪智慧".to_string(),
            url: "https://www.cdstm.cn".to_string(),
            icon: "🔬".to_string(),
            category: "科普探索".to_string(),
            enabled: true,
        },
        RecommendedSite {
            id: "4".to_string(),
            title: " Scratch".to_string(),
            description: "用积木学编程".to_string(),
            url: "https://scratch.mit.edu".to_string(),
            icon: "💻".to_string(),
            category: "编程学习".to_string(),
            enabled: true,
        },
        RecommendedSite {
            id: "5".to_string(),
            title: "编程猫".to_string(),
            description: "青少年编程学习平台".to_string(),
            url: "https://codecat.cn".to_string(),
            icon: "🐱".to_string(),
            category: "编程学习".to_string(),
            enabled: true,
        },
        RecommendedSite {
            id: "6".to_string(),
            title: "故宫博物馆".to_string(),
            description: "探索中华文化瑰宝".to_string(),
            url: "https://www.dpm.org.cn".to_string(),
            icon: "🏛️".to_string(),
            category: "艺术文化".to_string(),
            enabled: true,
        },
        RecommendedSite {
            id: "7".to_string(),
            title: "Bilibili 学习".to_string(),
            description: "海量优质学习视频".to_string(),
            url: "https://www.bilibili.com".to_string(),
            icon: "📺".to_string(),
            category: "艺术文化".to_string(),
            enabled: true,
        },
        RecommendedSite {
            id: "8".to_string(),
            title: " WWF中国".to_string(),
            description: "保护地球生物多样性".to_string(),
            url: "https://www.wwfchina.org".to_string(),
            icon: "🌿".to_string(),
            category: "自然生态".to_string(),
            enabled: true,
        },
        RecommendedSite {
            id: "9".to_string(),
            title: "中国科普博览".to_string(),
            description: "走进科学的殿堂".to_string(),
            url: "https://www.kepu.cn".to_string(),
            icon: "📚".to_string(),
            category: "科普探索".to_string(),
            enabled: true,
        },
        RecommendedSite {
            id: "10".to_string(),
            title: "网易公开课".to_string(),
            description: "世界名校精品课程".to_string(),
            url: "https://open.163.com".to_string(),
            icon: "🎓".to_string(),
            category: "科普探索".to_string(),
            enabled: true,
        },
    ]
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}-{:x}", (now >> 64) as u64, now as u64)
}

impl TimeSlot {
    pub fn is_active_now(&self) -> bool {
        use std::time::SystemTime;

        let now = SystemTime::now();
        let datetime: chrono::DateTime<chrono::Local> = chrono::DateTime::from(now);
        let weekday = datetime.format("%w").to_string().parse::<u8>().unwrap_or(0);

        if !self.days.contains(&weekday) {
            return false;
        }

        let time = datetime.time();
        if let (Ok(start), Ok(end)) = (
            chrono::NaiveTime::parse_from_str(&self.start, "%H:%M"),
            chrono::NaiveTime::parse_from_str(&self.end, "%H:%M"),
        ) {
            if start <= end {
                time >= start && time <= end
            } else {
                time >= start || time <= end
            }
        } else {
            false
        }
    }
}

impl TimeRule {
    pub fn is_access_allowed(&self) -> (bool, String) {
        if !self.enabled {
            return (true, String::new());
        }

        use chrono::Local;
        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();

        if self.usage_today.date != today {
            if self.usage_today.used_minutes >= self.max_daily_minutes {
                return (
                    false,
                    format!("今日上网时长已达上限 ({} 分钟)", self.max_daily_minutes),
                );
            }
        } else if self.usage_today.used_minutes >= self.max_daily_minutes {
            return (
                false,
                format!("今日上网时长已达上限 ({} 分钟)", self.max_daily_minutes),
            );
        }

        let active_slot = self.slots.iter().find(|s| s.is_active_now());
        if let Some(_slot) = active_slot {
            (true, String::new())
        } else if self.slots.is_empty() {
            (true, String::new())
        } else {
            let allowed_times: Vec<String> = self
                .slots
                .iter()
                .map(|s| format!("{}:{}", s.start, s.end))
                .collect();
            (
                false,
                format!(
                    "当前不在允许上网时段内。允许时段: {}",
                    allowed_times.join(", ")
                ),
            )
        }
    }

    pub fn add_usage_minutes(&mut self, minutes: u32) {
        use chrono::Local;
        let today = Local::now().format("%Y-%m-%d").to_string();
        if self.usage_today.date != today {
            self.usage_today = UsageRecord {
                date: today,
                used_minutes: minutes,
            };
        } else {
            self.usage_today.used_minutes += minutes;
        }
    }

    pub fn remaining_minutes(&self) -> u32 {
        use chrono::Local;
        let today = Local::now().format("%Y-%m-%d").to_string();
        if self.usage_today.date != today {
            self.max_daily_minutes
        } else {
            self.max_daily_minutes
                .saturating_sub(self.usage_today.used_minutes)
        }
    }
}

impl UrlRule {
    pub fn matches(&self, domain: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let pattern = &self.pattern;
        if pattern.is_empty() {
            return false;
        }

        if pattern.starts_with("*.") {
            let suffix = &pattern[2..];
            domain.ends_with(suffix) || domain == suffix
        } else if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len() - 1];
            domain.contains(middle)
        } else if pattern.starts_with('*') {
            domain.ends_with(&pattern[1..])
        } else if pattern.ends_with('*') {
            domain.starts_with(&pattern[..pattern.len() - 1])
        } else {
            domain.contains(pattern)
        }
    }
}

impl FilterConfig {
    pub fn check_url(&self, url: &str, domain: &str) -> (bool, Option<String>) {
        log::debug!(
            "🔍 [FilterConfig] 检查 URL: {} | 域名: {} | 规则数量: {}",
            url,
            domain,
            self.url_rules.len()
        );

        for rule in &self.url_rules {
            if !rule.enabled {
                continue;
            }

            log::debug!("  检查规则: {} (pattern: {})", rule.name, rule.pattern);

            if rule.matches(domain) {
                log::info!(
                    "🚫 [FilterConfig] 规则匹配成功: {} - 域名: {} - 模式: {}",
                    rule.name,
                    domain,
                    rule.pattern
                );
                return (false, Some(rule.name.clone()));
            }
            if !rule.pattern.is_empty() && url.contains(&rule.pattern) {
                log::info!(
                    "🚫 [FilterConfig] URL包含规则: {} - URL: {} - 模式: {}",
                    rule.name,
                    url,
                    rule.pattern
                );
                return (false, Some(rule.name.clone()));
            }
        }
        (true, None)
    }

    pub fn is_blocked_category(&self, category: &str) -> bool {
        self.block_categories.contains(category)
    }
}
