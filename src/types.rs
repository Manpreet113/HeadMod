use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use dashmap::DashMap;
use poise::serenity_prelude::{UserId, GuildId, ChannelId};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Clone)]
pub struct Data {
    pub db: SqlitePool,
    pub config_cache: Arc<RwLock<HashMap<u64, GuildConfig>>>,
    pub channel_cache: Arc<RwLock<HashMap<u64, ChannelConfig>>>,
    pub message_counts: Arc<DashMap<UserId, Vec<chrono::DateTime<chrono::Utc>>>>,
    pub join_counts: Arc<DashMap<GuildId, Vec<chrono::DateTime<chrono::Utc>>>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct GuildConfig {
    pub guild_id: i64,
    pub mod_log_channel_id: Option<i64>,
    pub message_log_channel_id: Option<i64>,
    pub warn_threshold: i64,
    pub warn_timeout_secs: i64,
    pub anti_invite: bool,
    pub anti_spam: bool,
    pub retention_days: i64,
    pub anti_caps: bool,
    pub min_account_age_days: i64,
    pub ticket_channel_id: Option<i64>,
    pub ticket_mod_role_id: Option<i64>,
    pub anti_evasion: bool,
    pub toxicity_threshold: i64,
    pub evidence_channel_id: Option<i64>,
    pub verification_channel_id: Option<i64>,
    pub verified_role_id: Option<i64>,
    pub last_summary_at: Option<chrono::NaiveDateTime>,
    pub join_log_channel_id: Option<i64>,
    pub leave_log_channel_id: Option<i64>,
    pub suspicious_log_channel_id: Option<i64>,
    pub global_intel_enabled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct ChannelConfig {
    pub channel_id: i64,
    pub guild_id: i64,
    pub toxicity_threshold: i64,
    pub is_relaxed: bool,
}

impl Data {
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
            config_cache: Arc::new(RwLock::new(HashMap::new())),
            channel_cache: Arc::new(RwLock::new(HashMap::new())),
            message_counts: Arc::new(DashMap::new()),
            join_counts: Arc::new(DashMap::new()),
        }
    }

    pub async fn get_config(&self, guild_id: GuildId) -> Option<GuildConfig> {
        let gid = guild_id.get() as i64;
        {
            let cache = self.config_cache.read().await;
            if let Some(config) = cache.get(&(guild_id.get())) {
                return Some(config.clone());
            }
        }

        let config = sqlx::query_as!(
            GuildConfig,
            r#"
            SELECT 
                guild_id as "guild_id!: i64", 
                mod_log_channel_id, 
                message_log_channel_id, 
                warn_threshold as "warn_threshold!: i64", 
                warn_timeout_secs as "warn_timeout_secs!: i64", 
                anti_invite as "anti_invite!: bool", 
                anti_spam as "anti_spam!: bool", 
                retention_days as "retention_days!: i64", 
                anti_caps as "anti_caps!: bool", 
                min_account_age_days as "min_account_age_days!: i64", 
                ticket_channel_id, 
                ticket_mod_role_id, 
                anti_evasion as "anti_evasion!: bool", 
                toxicity_threshold as "toxicity_threshold!: i64", 
                evidence_channel_id, 
                verification_channel_id, 
                verified_role_id, 
                last_summary_at as "last_summary_at: chrono::NaiveDateTime",
                join_log_channel_id,
                leave_log_channel_id,
                suspicious_log_channel_id,
                global_intel_enabled as "global_intel_enabled!: bool"
            FROM guild_configs WHERE guild_id = ?
            "#,
            gid
        ).fetch_optional(&self.db).await.unwrap_or(None);

        if let Some(ref c) = config {
            let mut cache = self.config_cache.write().await;
            cache.insert(guild_id.get(), c.clone());
        }
        config
    }

    pub async fn get_channel_config(&self, channel_id: ChannelId) -> Option<ChannelConfig> {
        let cid = channel_id.get() as i64;
        {
            let cache = self.channel_cache.read().await;
            if let Some(config) = cache.get(&(channel_id.get())) {
                return Some(config.clone());
            }
        }

        let config = sqlx::query_as!(
            ChannelConfig,
            r#"
            SELECT 
                channel_id as "channel_id!: i64",
                guild_id as "guild_id!: i64",
                toxicity_threshold as "toxicity_threshold!: i64",
                is_relaxed as "is_relaxed!: bool"
            FROM channel_configs WHERE channel_id = ?
            "#,
            cid
        ).fetch_optional(&self.db).await.unwrap_or(None);

        if let Some(ref c) = config {
            let mut cache = self.channel_cache.write().await;
            cache.insert(channel_id.get(), c.clone());
        }
        config
    }
}