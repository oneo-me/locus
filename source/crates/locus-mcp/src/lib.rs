#![forbid(unsafe_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::thread::{self, JoinHandle};

mod service;

use axum::Router;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio_util::sync::CancellationToken;

use service::LocusMcp;

const MCP_PATH: &str = "/mcp";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum McpServerState {
    Stopped,
    Running,
    Failed { message: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpConfiguration {
    pub library_path: Option<PathBuf>,
    pub enabled: bool,
    pub local_network: bool,
    pub port: u16,
}

pub struct McpServerController {
    cancellation: Option<CancellationToken>,
    thread: Option<JoinHandle<()>>,
    state: McpServerState,
}

impl McpServerController {
    pub fn new(configuration: &McpConfiguration) -> Self {
        let mut controller = Self {
            cancellation: None,
            thread: None,
            state: McpServerState::Stopped,
        };
        controller.apply(configuration);
        controller
    }

    pub fn apply(&mut self, configuration: &McpConfiguration) -> McpServerState {
        self.stop();
        if !configuration.enabled {
            self.state = McpServerState::Stopped;
            return self.state.clone();
        }

        self.state = match self.start(configuration) {
            Ok(()) => McpServerState::Running,
            Err(message) => McpServerState::Failed { message },
        };
        self.state.clone()
    }

    pub fn state(&self) -> McpServerState {
        self.state.clone()
    }

    fn start(&mut self, configuration: &McpConfiguration) -> Result<(), String> {
        let address = bind_address(configuration.local_network, configuration.port);
        let listener =
            TcpListener::bind(address).map_err(|error| format!("无法监听 {address}：{error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("无法配置 MCP 端口：{error}"))?;

        let cancellation = CancellationToken::new();
        let server_cancellation = cancellation.clone();
        let root = configuration.library_path.clone();
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
                            .with_cancellation_token(server_cancellation.child_token()),
                    );
                    let make_service = tower::service_fn(
                        move |stream: axum::serve::IncomingStream<'_, tokio::net::TcpListener>| {
                            let hosts = allowed_hosts(stream.io().local_addr().ok());
                            let mut service = service.clone();
                            service.config.allowed_hosts = hosts;
                            let router = Router::new().route_service(MCP_PATH, service);
                            std::future::ready(Ok::<_, std::convert::Infallible>(router))
                        },
                    );
                    let shutdown = server_cancellation.clone();
                    if let Err(error) = axum::serve(listener, make_service)
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
        Ok(())
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

fn bind_address(local_network: bool, port: u16) -> SocketAddr {
    let ip = if local_network {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    } else {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    };
    SocketAddr::new(ip, port)
}

fn allowed_hosts(local_address: Option<SocketAddr>) -> Vec<String> {
    let mut hosts = vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ];
    if let Some(address) = local_address {
        let host = address.ip().to_string();
        if !hosts.contains(&host) {
            hosts.push(host);
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
    fn listener_reflects_binding_scope() {
        assert_eq!(
            bind_address(false, 3001),
            SocketAddr::from((Ipv4Addr::LOCALHOST, 3001))
        );
        assert_eq!(
            bind_address(true, 3001),
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3001))
        );
    }

    #[test]
    fn allowed_hosts_follow_the_connection_destination() {
        let first = allowed_hosts(Some("192.168.1.24:2301".parse().unwrap()));
        assert_eq!(first, ["localhost", "127.0.0.1", "::1", "192.168.1.24"]);
        let second = allowed_hosts(Some("10.0.0.8:2301".parse().unwrap()));
        assert_eq!(second, ["localhost", "127.0.0.1", "::1", "10.0.0.8"]);
        assert_eq!(allowed_hosts(None), ["localhost", "127.0.0.1", "::1"]);
        assert_eq!(
            allowed_hosts(Some("127.0.0.1:2301".parse().unwrap())),
            ["localhost", "127.0.0.1", "::1"]
        );
    }

    #[test]
    fn configuration_changes_stop_the_listener_and_report_bind_failures() {
        let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = occupied.local_addr().unwrap().port();
        let mut configuration = McpConfiguration {
            library_path: None,
            enabled: false,
            local_network: false,
            port,
        };
        let mut controller = McpServerController::new(&configuration);
        assert_eq!(controller.state(), McpServerState::Stopped);

        configuration.enabled = true;
        assert!(matches!(
            controller.apply(&configuration),
            McpServerState::Failed { .. }
        ));
        drop(occupied);
        assert!(matches!(
            controller.apply(&configuration),
            McpServerState::Running
        ));
        assert!(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_err());

        configuration.enabled = false;
        assert!(matches!(
            controller.apply(&configuration),
            McpServerState::Stopped
        ));
        let released = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
        drop(released);
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

        let configuration = McpConfiguration {
            library_path: Some(library.0.clone()),
            enabled: true,
            port,
            local_network: false,
        };
        let controller = McpServerController::new(&configuration);
        assert!(matches!(controller.state(), McpServerState::Running));

        let transport =
            StreamableHttpClientTransport::from_uri(format!("http://127.0.0.1:{port}/mcp"));
        let client = ().serve(transport).await.unwrap();
        assert_eq!(
            client.peer_info().unwrap().protocol_version,
            rmcp::model::ProtocolVersion::V_2025_11_25
        );
        assert!(client.peer_info().unwrap().capabilities.resources.is_none());
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

        for path in ["notes", "locus://notes", "locus://notes/index.md"] {
            let resolved = client
                .call_tool(
                    CallToolRequestParams::new("resolve_article")
                        .with_arguments(json!({"path": path}).as_object().unwrap().clone()),
                )
                .await
                .unwrap();
            assert_eq!(resolved.is_error, Some(false));
            assert_eq!(resolved.structured_content.unwrap(), *article);
        }
        for parent in ["locus://", "locus://notes"] {
            let listed = client
                .call_tool(
                    CallToolRequestParams::new("list_articles")
                        .with_arguments(json!({"parent": parent}).as_object().unwrap().clone()),
                )
                .await
                .unwrap();
            assert_eq!(listed.is_error, Some(false));
            let listed = listed.structured_content.unwrap();
            if parent == "locus://" {
                assert_eq!(listed["articles"][0], *article);
                assert_eq!(listed["parent"], "");
            } else {
                assert_eq!(listed["parent"], "notes");
                assert_eq!(listed["articles"], json!([]));
            }
        }

        client.cancel().await.unwrap();
        let removed =
            StreamableHttpClientTransport::from_uri(format!("http://127.0.0.1:{port}/mcp/full"));
        assert!(().serve(removed).await.is_err());
        drop(controller);
    }
}
