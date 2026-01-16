pub mod entity;
pub mod repository;

pub struct DatabaseManager;

use kernel::config::database_config;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

// 使用 OnceLock 存储数据库连接池
static DB_POOL: OnceLock<Arc<DatabaseConnection>> = OnceLock::new();

impl DatabaseManager {
    /// 初始化全局数据库连接（应用启动时调用）
    pub async fn init() -> Result<(), sea_orm::DbErr> {
        let config = database_config();

        let mut opt = ConnectOptions::new(config.database_url.to_owned());
        opt.max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect_timeout(Duration::from_secs(config.connect_timeout_seconds as u64))
            .idle_timeout(Duration::from_secs(config.connect_timeout_seconds as u64))
            .sqlx_logging(false);

        let connection = Database::connect(opt).await?;
        DB_POOL
            .set(Arc::new(connection))
            .map_err(|_| sea_orm::DbErr::Custom("DB already initialized".to_string()))?;
        Ok(())
    }

    /// 获取数据库连接（可在任何地方调用）
    pub fn get() -> Option<Arc<DatabaseConnection>> {
        DB_POOL.get().cloned()
    }

    /// 获取数据库连接，如果未初始化则panic
    pub fn get_unwrap() -> Arc<DatabaseConnection> {
        DB_POOL.get().cloned().expect("Database not initialized")
    }
}

// 为了方便使用，提供全局函数
pub fn get_db() -> Option<Arc<DatabaseConnection>> {
    DatabaseManager::get()
}

pub fn get_db_unwrap() -> &'static DatabaseConnection {
    unsafe { &*Arc::as_ptr(DB_POOL.get().expect("Database not initialized")) }
}
