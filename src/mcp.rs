use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket};
use std::thread::{self, JoinHandle};

mod service;

use axum::Router;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio_util::sync::CancellationToken;

use crate::preferences::Preferences;
use service::LocusMcp;

const MCP_PATH: &str = "/mcp";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum McpServerState {
    Stopped,
    Running { endpoint: String },
    Failed { message: String },
}

pub struct McpServerController {
    cancellation: Option<CancellationToken>,
    thread: Option<JoinHandle<()>>,
    state: McpServerState,
}

impl McpServerController {
    pub fn new(preferences: &Preferences) -> Self {
        let mut controller = Self {
            cancellation: None,
            thread: None,
            state: McpServerState::Stopped,
        };
        controller.apply(preferences);
        controller
    }

    pub fn apply(&mut self, preferences: &Preferences) -> McpServerState {
        self.stop();
        if !preferences.mcp_enabled {
            self.state = McpServerState::Stopped;
            return self.state.clone();
        }

        self.state = match self.start(preferences) {
            Ok(endpoint) => McpServerState::Running { endpoint },
            Err(message) => McpServerState::Failed { message },
        };
        self.state.clone()
    }

    pub fn state(&self) -> McpServerState {
        self.state.clone()
    }

    fn start(&mut self, preferences: &Preferences) -> Result<String, String> {
        let address = bind_address(preferences.mcp_local_network, preferences.mcp_port);
        let listener =
            TcpListener::bind(address).map_err(|error| format!("无法监听 {address}：{error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("无法配置 MCP 端口：{error}"))?;

        let cancellation = CancellationToken::new();
        let server_cancellation = cancellation.clone();
        let root = preferences.library_path.clone();
        let allowed_hosts = allowed_hosts(preferences.mcp_local_network);
        let thread = thread::Builder::new()
            .name("locus-mcp".to_owned())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        eprintln!("无法启动 MCP 运行时：{error}");
                        return;
                    }
                };
                runtime.block_on(async move {
                    let listener = match tokio::net::TcpListener::from_std(listener) {
                        Ok(listener) => listener,
                        Err(error) => {
                            eprintln!("无法启动 MCP 监听器：{error}");
                            return;
                        }
                    };
                    let service_root = root.clone();
                    let service = StreamableHttpService::new(
                        move || Ok(LocusMcp::new(service_root.clone())),
                        LocalSessionManager::default().into(),
                        StreamableHttpServerConfig::default()
                            .with_allowed_hosts(allowed_hosts)
                            .with_cancellation_token(server_cancellation.child_token()),
                    );
                    let router = Router::new().nest_service(MCP_PATH, service);
                    let shutdown = server_cancellation.clone();
                    if let Err(error) = axum::serve(listener, router)
                        .with_graceful_shutdown(shutdown.cancelled_owned())
                        .await
                    {
                        eprintln!("MCP 服务已停止：{error}");
                    }
                });
            })
            .map_err(|error| format!("无法创建 MCP 服务线程：{error}"))?;

        self.cancellation = Some(cancellation);
        self.thread = Some(thread);
        Ok(mcp_endpoint(
            preferences.mcp_local_network,
            preferences.mcp_port,
        ))
    }

    fn stop(&mut self) {
        if let Some(cancellation) = self.cancellation.take() {
            cancellation.cancel();
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("MCP 服务线程异常退出。");
        }
    }
}

impl Drop for McpServerController {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn mcp_endpoint(local_network: bool, port: u16) -> String {
    let host = if local_network {
        local_network_ip()
            .map(|address| address.to_string())
            .unwrap_or_else(|| "<本机局域网 IP>".to_owned())
    } else {
        "127.0.0.1".to_owned()
    };
    format!("http://{host}:{port}{MCP_PATH}")
}

fn bind_address(local_network: bool, port: u16) -> SocketAddr {
    let ip = if local_network {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    } else {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    };
    SocketAddr::new(ip, port)
}

fn local_network_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 80)).ok()?;
    let address = socket.local_addr().ok()?.ip();
    (!address.is_loopback() && !address.is_unspecified()).then_some(address)
}

fn allowed_hosts(local_network: bool) -> Vec<String> {
    let mut hosts = vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ];
    if local_network {
        hosts.push("0.0.0.0".to_owned());
        if let Some(address) = local_network_ip() {
            hosts.push(address.to_string());
        }
    }
    hosts
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use rmcp::{
        ServiceExt, model::CallToolRequestParams, transport::StreamableHttpClientTransport,
    };
    use serde_json::json;

    use super::*;

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestLibrary(PathBuf);

    impl TestLibrary {
        fn new() -> Self {
            let id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("locus-mcp-{}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestLibrary {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn endpoint_reflects_binding_scope() {
        assert_eq!(
            bind_address(false, 3001),
            SocketAddr::from((Ipv4Addr::LOCALHOST, 3001))
        );
        assert_eq!(
            bind_address(true, 3001),
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3001))
        );
        assert_eq!(mcp_endpoint(false, 3001), "http://127.0.0.1:3001/mcp");
        assert!(mcp_endpoint(true, 3001).ends_with(":3001/mcp"));
    }

    #[tokio::test]
    async fn streamable_http_exposes_the_three_read_only_tools() {
        let library = TestLibrary::new();
        let article = library.0.join("notes");
        fs::create_dir(&article).unwrap();
        fs::write(
            article.join("index.md"),
            "---\ntitle: Notes\nsummary: Searchable metadata\n---\nFirst line\nSecond line\n",
        )
        .unwrap();
        let reserved = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = reserved.local_addr().unwrap().port();
        drop(reserved);

        let preferences = Preferences {
            library_path: Some(library.0.clone()),
            mcp_enabled: true,
            mcp_port: port,
            ..Preferences::default()
        };
        let controller = McpServerController::new(&preferences);
        assert!(matches!(controller.state(), McpServerState::Running { .. }));

        let transport =
            StreamableHttpClientTransport::from_uri(format!("http://127.0.0.1:{port}/mcp"));
        let client = ().serve(transport).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();
        let names = tools
            .iter()
            .map(|tool| tool.name.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            ["list_articles", "resolve_article", "search_articles"]
        );
        assert!(tools.iter().all(|tool| {
            tool.annotations
                .as_ref()
                .is_some_and(|annotations| annotations.read_only_hint == Some(true))
        }));

        let result = client
            .call_tool(
                CallToolRequestParams::new("list_articles")
                    .with_arguments(json!({}).as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        let article = &result.structured_content.unwrap()["articles"][0];
        assert_eq!(article["path"], "notes");
        assert_eq!(article["lineCount"], 2);
        assert_eq!(article["childCount"], 0);

        client.cancel().await.unwrap();
        drop(controller);
    }
}
