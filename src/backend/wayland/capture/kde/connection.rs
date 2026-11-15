//! 【KWin捕获】【运行时连接】总线接收任务与创建连接时的异步运行时保持一致

use tokio::sync::Mutex;
use zbus::Connection;

#[derive(Default)]
pub(super) struct RuntimeConnection {
    cached: Mutex<Option<(tokio::runtime::Id, Connection)>>,
}

impl RuntimeConnection {
    /// 【KWin捕获】【总线获取】无参数，返回当前运行时的连接，同运行时复用，后台移交后重新连接
    pub(super) async fn get(&self) -> zbus::Result<Connection> {
        self.get_with(Connection::session).await
    }

    /// 【KWin捕获】【连接建立】按 connect 创建连接并返回可独立持有的句柄；连接工厂用于隔离总线测试
    async fn get_with<F, Fut>(&self, connect: F) -> zbus::Result<Connection>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = zbus::Result<Connection>>,
    {
        let runtime = tokio::runtime::Handle::current().id();
        let mut cached = self.cached.lock().await;
        if let Some((owner, connection)) = cached.as_ref()
            && *owner == runtime
        {
            return Ok(connection.clone());
        }
        // 1. 【KWin捕获】【后台移交】旧运行时可能已阻塞在原生输入，接收任务必须在当前运行时重新建立
        let connection = connect().await?;
        *cached = Some((runtime, connection.clone()));
        Ok(connection)
    }
}

#[cfg(test)]
#[path = "connection_tests.rs"]
mod tests;
