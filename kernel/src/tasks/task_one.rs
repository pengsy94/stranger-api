use tokio_cron_scheduler::{Job, JobSchedulerError, job::JobLocked};
use tracing::{error, info};

/// 实现示例任务
pub fn handle_task() -> Result<JobLocked, JobSchedulerError> {
    let jon = Job::new("*/10 * * * * *", |_uuid, _l| {
        info!(
            "示例任务执行 - 时间: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );
    });

    // 预先处理掉错误的情况
    if let Err(e) = &jon {
        error!("{}", e);
    }

    jon
}
