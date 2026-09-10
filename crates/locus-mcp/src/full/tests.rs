use super::*;
use crate::{McpConfiguration, McpServerController, McpServerState};
use base64::Engine;
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    service::{RoleClient, RunningService},
    transport::StreamableHttpClientTransport,
};

async fn call(
    client: &RunningService<RoleClient, ()>,
    name: &str,
    arguments: Value,
) -> CallToolResult {
    client
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(arguments.as_object().unwrap().clone()),
        )
        .await
        .unwrap()
}

fn success(result: CallToolResult) -> Value {
    assert_ne!(result.is_error, Some(true), "{result:?}");
    result.structured_content.unwrap()
}

#[tokio::test]
async fn complete_library_workflow_uses_only_the_full_http_endpoint() {
    let root = tempfile::tempdir().unwrap();
    let reserved = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = reserved.local_addr().unwrap().port();
    drop(reserved);
    let configuration = McpConfiguration {
        library_path: Some(root.path().to_owned()),
        enabled: true,
        local_network: false,
        port,
    };
    let controller = McpServerController::new(&configuration);
    assert!(matches!(controller.state(), McpServerState::Running { .. }));
    let client = ()
        .serve(StreamableHttpClientTransport::from_uri(format!(
            "http://127.0.0.1:{port}/mcp/full"
        )))
        .await
        .unwrap();
    let tools = client.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 22);
    assert!(tools.iter().any(|tool| tool.name == "upload_chunk"));
    assert!(
        !client
            .peer_info()
            .unwrap()
            .instructions
            .as_ref()
            .unwrap()
            .contains("Edit content with the client file tools")
    );
    let standard = ()
        .serve(StreamableHttpClientTransport::from_uri(format!(
            "http://127.0.0.1:{port}/mcp"
        )))
        .await
        .unwrap();
    assert_eq!(standard.list_all_tools().await.unwrap().len(), 3);
    let uri = "locus://旅行";
    success(call(&client, "create_note", json!({"uri":uri,"markdown":"---\ntitle: Travel\ncustom: [a, b]\n---\nFirst\nSecond\n"})).await);
    let note = json!({"uri":uri,"resource":{"kind":"note"}});
    let first = success(call(&client, "read_text", json!({"target":note,"limit":2})).await);
    assert_eq!(first["nextLine"], 2);
    let read = success(call(&client, "read_text", json!({"target":note})).await);
    success(call(&client, "patch_text", json!({"target":note,"expected_version":read["version"],"old_text":"First","new_text":"Updated"})).await);
    assert_eq!(
        call(
            &client,
            "write_text",
            json!({"target":note,"text":"Stale","expected_version":read["version"]})
        )
        .await
        .is_error,
        Some(true)
    );
    let attachment = json!({"uri":uri,"resource":{"kind":"attachment","name":"data.bin"}});
    let bytes: Vec<u8> = (0..150_000).map(|i| (i % 256) as u8).collect();
    let begin = success(call(&client, "begin_upload", json!({"target":attachment,"size":bytes.len(),"sha256":locus_library::content_version(&bytes),"expected_version":"missing"})).await);
    assert_eq!(
        call(
            &client,
            "finish_upload",
            json!({"upload_id":begin["uploadId"]})
        )
        .await
        .is_error,
        Some(true)
    );
    for (ix, chunk) in bytes.chunks(64 * 1024).enumerate() {
        let params = json!({"upload_id":begin["uploadId"],"offset":ix*64*1024,"base64":base64::engine::general_purpose::STANDARD.encode(chunk)});
        success(call(&client, "upload_chunk", params.clone()).await);
        success(call(&client, "upload_chunk", params).await);
    }
    let finished = success(
        call(
            &client,
            "finish_upload",
            json!({"upload_id":begin["uploadId"]}),
        )
        .await,
    );
    assert_eq!(finished["version"], locus_library::content_version(&bytes));
    let mut downloaded = Vec::new();
    loop {
        let chunk = success(call(&client, "download_chunk", json!({"target":attachment,"version":finished["version"],"offset":downloaded.len()})).await);
        downloaded.extend(
            base64::engine::general_purpose::STANDARD
                .decode(chunk["base64"].as_str().unwrap())
                .unwrap(),
        );
        if chunk["eof"] == true {
            break;
        }
    }
    assert_eq!(downloaded, bytes);
    let renamed = json!({"uri":uri,"resource":{"kind":"attachment","name":"renamed.bin"}});
    success(call(&client, "move_attachment", json!({"source":attachment,"destination":renamed,"expected_version":finished["version"]})).await);
    let canvas = json!({"uri":uri,"resource":{"kind":"canvas"}});
    success(
        call(
            &client,
            "write_text",
            json!({"target":canvas,"text":"<p>Canvas</p>","expected_version":"missing"}),
        )
        .await,
    );
    assert_eq!(
        success(call(&client, "read_text", json!({"target":canvas})).await)["text"],
        "<p>Canvas</p>"
    );
    let note_version =
        success(call(&client, "stat_content", json!({"target":note})).await)["version"].clone();
    let archived = success(
        call(
            &client,
            "set_archived",
            json!({"uri":uri,"expected_version":note_version,"archived":true}),
        )
        .await,
    );
    assert_eq!(
        success(call(&client, "list_entries", json!({})).await)["entries"][0]["metadata"]["archived"],
        true
    );
    success(
        call(
            &client,
            "set_archived",
            json!({"uri":uri,"expected_version":archived["version"],"archived":false}),
        )
        .await,
    );
    let manifest = success(call(&client, "export_manifest", json!({"uri":uri})).await);
    assert_eq!(manifest["entries"].as_array().unwrap().len(), 4);
    assert!(!manifest.to_string().contains(root.path().to_str().unwrap()));
    let plan = success(
        call(
            &client,
            "plan_move",
            json!({"uri":uri,"destination":"locus://renamed"}),
        )
        .await,
    );
    success(
        call(
            &client,
            "move_entry",
            json!({"uri":uri,"destination":"locus://renamed","expected_version":plan["version"]}),
        )
        .await,
    );
    assert_eq!(
        success(call(&client, "resolve_entry", json!({"uri":"locus://renamed"})).await)["metadata"]
            ["title"],
        "Travel"
    );
    let tree = success(call(&client, "inspect_tree", json!({"uri":"locus://renamed"})).await);
    success(
        call(
            &client,
            "delete_entry",
            json!({"uri":"locus://renamed","expected_version":tree["version"],"recursive":true}),
        )
        .await,
    );
    assert_eq!(
        success(call(&client, "list_entries", json!({})).await)["entries"],
        json!([])
    );
    assert_eq!(
        call(
            &client,
            "create_note",
            json!({"uri":"locus://../outside","markdown":"Bad"})
        )
        .await
        .is_error,
        Some(true)
    );
    standard.cancel().await.unwrap();
    client.cancel().await.unwrap();
    drop(controller);
}

#[tokio::test]
async fn stopping_service_invalidates_old_library_access() {
    let root = tempfile::tempdir().unwrap();
    let library = Arc::new(Mutex::new(LibraryAccess::new(Some(root.path().to_owned()))));
    let cancellation = CancellationToken::new();
    let service = FullMcp::new(library, cancellation.clone());
    cancellation.cancel();
    let result = service
        .create_note(Parameters(CreateNote {
            uri: "locus://note".into(),
            markdown: "Bad".into(),
        }))
        .await;
    assert_eq!(result.is_error, Some(true));
    assert!(!root.path().join("note").exists());
}

#[tokio::test]
async fn local_network_mode_shares_both_routes_and_rejects_unknown_hosts() {
    use std::io::{Read, Write};
    let root = tempfile::tempdir().unwrap();
    let reserved = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = reserved.local_addr().unwrap().port();
    drop(reserved);
    let configuration = McpConfiguration {
        library_path: Some(root.path().to_owned()),
        enabled: true,
        local_network: true,
        port,
    };
    let controller = McpServerController::new(&configuration);
    assert!(matches!(controller.state(), McpServerState::Running { .. }));
    assert!(
        controller
            .state()
            .full_endpoint()
            .ends_with(&format!(":{port}/mcp/full"))
    );
    for route in ["/mcp", "/mcp/full"] {
        let client = ()
            .serve(StreamableHttpClientTransport::from_uri(format!(
                "http://127.0.0.1:{port}{route}"
            )))
            .await
            .unwrap();
        assert!(!client.list_all_tools().await.unwrap().is_empty());
        client.cancel().await.unwrap();
        let mut connection =
            std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
        connection
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        write!(connection, "POST {route} HTTP/1.1\r\nHost: untrusted.example\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}").unwrap();
        let mut response = String::new();
        connection.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 403"), "{response}");
    }
    drop(controller);
}
