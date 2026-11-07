//! 【持续捕获回归】【Portal服务】实现真实D-Bus协议请求并提供私有PipeWire远端，记录资源生命周期

use std::{
    collections::{BTreeMap, HashMap},
    os::{fd::OwnedFd, unix::net::UnixStream},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use zbus::{
    interface,
    message::Header,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

type Dictionary = HashMap<String, OwnedValue>;

#[derive(Clone)]
struct Trace {
    path: PathBuf,
    counts: Arc<Mutex<BTreeMap<String, u64>>>,
}

impl Trace {
    /// 【持续捕获回归】【计数记录】按 action 增加trace中真实请求次数并保存JSON，返回协议结果
    fn add(&self, action: &str) -> zbus::fdo::Result<()> {
        let mut counts = self.counts.lock().unwrap();
        *counts.entry(action.into()).or_default() += 1;
        std::fs::write(&self.path, serde_json::to_vec_pretty(&*counts).unwrap())
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }
}

struct Request;

#[interface(name = "org.freedesktop.portal.Request")]
impl Request {
    /// 【持续捕获回归】【请求关闭】响应测试客户端显式关闭，无参数、无返回值
    fn close(&self) {}
}

struct Session {
    trace: Trace,
}

#[interface(name = "org.freedesktop.portal.Session")]
impl Session {
    /// 【持续捕获回归】【授权关闭】记录真实Close请求，无参数，返回计数写入结果
    fn close(&self) -> zbus::fdo::Result<()> {
        self.trace.add("close")
    }
    /// 【持续捕获回归】【会话版本】返回协议版本一，无参数
    #[zbus(property)]
    fn version(&self) -> u32 {
        1
    }
}

struct Portal {
    node: u32,
    socket: PathBuf,
    trace: Trace,
    invalid_restore: bool,
    reject_sources: bool,
    cancelled_sources: bool,
}

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.portal.Error")]
enum PortalFailure {
    InvalidArgument(String),
    Failed(String),
}

/// 【持续捕获回归】【协议字符串】按options和key读取请求字段，缺失或错误类型返回明确D-Bus错误
fn string<'a>(options: &'a Dictionary, key: &str) -> zbus::fdo::Result<&'a str> {
    options
        .get(key)
        .ok_or_else(|| zbus::fdo::Error::InvalidArgs(key.into()))?
        .downcast_ref::<&str>()
        .map_err(|error| zbus::fdo::Error::InvalidArgs(error.to_string()))
}

/// 【持续捕获回归】【协议路径】按header、kind和token生成实际调用方请求路径，返回有效对象路径
fn path(header: &Header<'_>, kind: &str, token: &str) -> zbus::fdo::Result<OwnedObjectPath> {
    let sender = header
        .sender()
        .ok_or_else(|| zbus::fdo::Error::Failed("sender required".into()))?
        .as_str()
        .trim_start_matches(':')
        .replace('.', "_");
    OwnedObjectPath::try_from(format!(
        "/org/freedesktop/portal/desktop/{kind}/{sender}/{token}"
    ))
    .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
}

/// 【持续捕获回归】【请求响应】按connection、header与options建立请求对象并发送result，返回实际请求路径
async fn respond(
    connection: &zbus::Connection,
    header: &Header<'_>,
    options: &Dictionary,
    result: Dictionary,
) -> zbus::fdo::Result<OwnedObjectPath> {
    response(connection, header, options, 0, result).await
}

/// 【持续捕获回归】【响应状态】按connection、header、options、code和result回传真实授权状态，返回请求路径
async fn response(
    connection: &zbus::Connection,
    header: &Header<'_>,
    options: &Dictionary,
    code: u32,
    result: Dictionary,
) -> zbus::fdo::Result<OwnedObjectPath> {
    let request = path(header, "request", string(options, "handle_token")?)?;
    connection
        .object_server()
        .at(request.clone(), Request)
        .await?;
    connection
        .emit_signal(
            None::<&str>,
            request.clone(),
            "org.freedesktop.portal.Request",
            "Response",
            &(code, result),
        )
        .await?;
    Ok(request)
}

#[interface(name = "org.freedesktop.portal.ScreenCast")]
impl Portal {
    /// 【持续捕获回归】【授权创建】以options和真实header创建会话，connection持有协议对象，返回请求路径
    async fn create_session(
        &self,
        options: Dictionary,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        self.trace.add("create")?;
        let session = path(
            &header,
            "session",
            string(&options, "session_handle_token")?,
        )?;
        connection
            .object_server()
            .at(
                session.clone(),
                Session {
                    trace: self.trace.clone(),
                },
            )
            .await?;
        respond(
            connection,
            &header,
            &options,
            HashMap::from([(
                "session_handle".into(),
                Value::from(session).try_to_owned().unwrap(),
            )]),
        )
        .await
    }

    /// 【持续捕获回归】【来源选择】核对options中的恢复授权并记录一次选择，session保持本次身份，返回请求路径
    async fn select_sources(
        &self,
        _session: OwnedObjectPath,
        options: Dictionary,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> Result<OwnedObjectPath, PortalFailure> {
        self.trace
            .add("select")
            .map_err(|error| PortalFailure::Failed(error.to_string()))?;
        if self.cancelled_sources {
            return response(connection, &header, &options, 1, HashMap::new())
                .await
                .map_err(|error| PortalFailure::Failed(error.to_string()));
        }
        if self.reject_sources || (self.invalid_restore && options.contains_key("restore_token")) {
            return Err(PortalFailure::InvalidArgument(
                "private test rejected the source selection".into(),
            ));
        }
        respond(connection, &header, &options, HashMap::new())
            .await
            .map_err(|error| PortalFailure::Failed(error.to_string()))
    }

    /// 【持续捕获回归】【授权启动】按options启动唯一来源并回传真实node，session和parent描述请求，返回请求路径
    async fn start(
        &self,
        _session: OwnedObjectPath,
        _parent: String,
        options: Dictionary,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        self.trace.add("start")?;
        let metadata = HashMap::from([
            (
                "position".to_owned(),
                Value::from((0i32, 0i32)).try_to_owned().unwrap(),
            ),
            (
                "size".to_owned(),
                Value::from((64i32, 32i32)).try_to_owned().unwrap(),
            ),
        ]);
        let streams = Value::from(vec![(self.node, metadata)])
            .try_to_owned()
            .unwrap();
        respond(
            connection,
            &header,
            &options,
            HashMap::from([
                ("streams".into(), streams),
                (
                    "restore_token".into(),
                    Value::from("private-continuous-token")
                        .try_to_owned()
                        .unwrap(),
                ),
            ]),
        )
        .await
    }

    /// 【持续捕获回归】【视频远端】按session和options提供本次私有PipeWire套接字，返回跨D-Bus文件描述符
    fn open_pipe_wire_remote(
        &self,
        _session: OwnedObjectPath,
        _options: Dictionary,
    ) -> zbus::fdo::Result<zbus::zvariant::OwnedFd> {
        self.trace.add("remote")?;
        let fd: OwnedFd = UnixStream::connect(&self.socket)
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?
            .into();
        Ok(fd.into())
    }

    /// 【持续捕获回归】【协议版本】返回支持恢复令牌的版本四，无参数
    #[zbus(property)]
    fn version(&self) -> u32 {
        4
    }
    /// 【持续捕获回归】【来源类型】只提供显示器来源，无参数，返回协议位掩码
    #[zbus(property)]
    fn available_source_types(&self) -> u32 {
        1
    }
    /// 【持续捕获回归】【指针类型】提供隐藏指针模式，无参数，返回协议位掩码
    #[zbus(property)]
    fn available_cursor_modes(&self) -> u32 {
        1
    }
}

/// 【持续捕获回归】【服务入口】按args中的node、socket和trace创建私有服务，返回设置或协议结果，服务生命周期由测试运行器管理
pub(super) async fn run(args: &[String]) -> mark_shot::AppResult {
    let node = args.first().ok_or("node required")?.parse()?;
    let socket = args.get(1).ok_or("socket required")?.into();
    let trace = Trace {
        path: args.get(2).ok_or("trace required")?.into(),
        counts: Arc::new(Mutex::new(BTreeMap::new())),
    };
    trace.add("server")?;
    let portal = Portal {
        node,
        socket,
        trace,
        invalid_restore: args.iter().any(|arg| arg == "invalid-restore"),
        reject_sources: args.iter().any(|arg| arg == "reject"),
        cancelled_sources: args.iter().any(|arg| arg == "source-cancel"),
    };
    let _connection = zbus::connection::Builder::session()?
        .name("org.freedesktop.portal.Desktop")?
        .serve_at("/org/freedesktop/portal/desktop", portal)?
        .build()
        .await?;
    println!("ready");
    std::future::pending::<()>().await;
    Ok(())
}
