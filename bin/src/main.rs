use bootstrap::logger;

mod bootstrap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 服务应用初始化
    let (make_service, listener, scheduler_manager) = bootstrap::make().await?;

    // 日志服务初始化(接收)
    let _logger = logger::Logger::init();

    tokio::select! {
        server_result =  axum::serve(listener, make_service) => {
            if let Err(e) = server_result {
                eprintln!("\n❌ 服务器异常错误: {}", e);
                std::process::exit(1);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n🕞 接收到 Ctrl+C 信号，正在优雅关闭...");

            // 调用调度器的关闭方法
            let shutdown_future = scheduler_manager.shutdown_future();
            shutdown_future.await;
        }
    }

    println!(
        "\n❌ Web服务已优雅关闭 [{}]\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    Ok(())
}
