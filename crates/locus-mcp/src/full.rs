use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use locus_library::{ContentTarget, LibraryAccess};
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

#[cfg(test)]
mod tests;
mod transfer;
use transfer::Transfers;

#[derive(Clone)]
pub(super) struct FullMcp {
    library: Arc<Mutex<LibraryAccess>>,
    transfers: Arc<Mutex<Transfers>>,
    cancellation: CancellationToken,
    tool_router: ToolRouter<Self>,
}

const INSTRUCTIONS: &str = "Use this endpoint for all library operations. Never request server file paths or ask the client to edit server files. Use locus:// logical note addresses; locus:// alone is the library root for listing/export. list_entries includes archived notes and organizational directories. resolve_entry lists note metadata and its note/canvas/icon/attachment resources. read_text returns raw UTF-8 (including YAML frontmatter) and a SHA-256 version. write_text replaces content; patch_text replaces one exact match, including metadata. Always pass the read version to writes/deletes, or 'missing' for creation. Resource targets are {uri, resource:{kind:'note'|'canvas'|'canvas_file'|'icon'|'attachment', name?:string}}. Attachments live in assets/; icon targets assets/icon.png. Canvas targets canvas/index.html; canvas_file with a relative name targets any other file under canvas/, including nested files. assets and canvas cannot be note names. create_note creates a note in an existing parent, preserving existing organizational children. Use set_archived for archive/restore, begin_upload/upload_chunk/finish_upload for any binary content, stat_content/download_chunk for byte-exact downloads. Upload chunks are base64 and at most 256 KiB decoded; complete size and optional SHA-256 are checked before commit. Icons are converted using the application's icon rules. Canvas content is the HTML fragment owned by a note, not a full HTML page; scripts/resources follow the application's canvas policy. Templates are other notes' canvas resources and are loaded at runtime with locus.loadCanvas('locus://...', data). export_manifest plus download_chunk exports a subtree; import by creating directories/notes parent-first and uploading each resource. Canvas and icon are distinct resources, not attachments. plan_move returns a version and Markdown link impact before move_entry renames/moves a complete tree. Dynamic canvas/script/custom-metadata references require explicit review and patch_text. move_attachment updates recognized Markdown links; review canvas/script references explicitly. inspect_tree returns the version needed for delete_entry; recursive=true explicitly deletes the entire subtree. Delete operations are permanent. Only this library is accessible; excluded paths and symlinks are rejected. Switching library or stopping the service invalidates sessions and transfers.";

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Browse {
    parent: Option<String>,
    cursor: Option<usize>,
    limit: Option<usize>,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Search {
    query: String,
    cursor: Option<usize>,
    limit: Option<usize>,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Entry {
    uri: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateNote {
    uri: String,
    markdown: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Target {
    target: ContentTarget,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReadText {
    target: ContentTarget,
    start_line: Option<usize>,
    limit: Option<usize>,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct WriteText {
    target: ContentTarget,
    text: String,
    expected_version: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PatchText {
    target: ContentTarget,
    old_text: String,
    new_text: String,
    expected_version: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Archive {
    uri: String,
    archived: bool,
    expected_version: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteContent {
    target: ContentTarget,
    expected_version: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MovePlan {
    uri: String,
    destination: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MoveEntry {
    uri: String,
    destination: String,
    expected_version: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MoveAttachment {
    source: ContentTarget,
    destination: ContentTarget,
    expected_version: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DeleteEntry {
    uri: String,
    expected_version: String,
    #[serde(default)]
    recursive: bool,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BeginUpload {
    target: ContentTarget,
    size: u64,
    sha256: Option<String>,
    expected_version: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UploadChunk {
    upload_id: String,
    offset: u64,
    base64: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct UploadId {
    upload_id: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DownloadChunk {
    target: ContentTarget,
    version: String,
    offset: u64,
    limit: Option<usize>,
}

impl FullMcp {
    pub(super) fn new(library: Arc<Mutex<LibraryAccess>>, cancellation: CancellationToken) -> Self {
        Self {
            library,
            transfers: Arc::new(Mutex::new(Transfers::default())),
            cancellation,
            tool_router: Self::tool_router(),
        }
    }

    async fn run(
        &self,
        operation: impl FnOnce(&LibraryAccess, &mut Transfers) -> Result<Value, String> + Send + 'static,
    ) -> CallToolResult {
        let service = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let library = service
                .library
                .lock()
                .map_err(|_| "service_unavailable: Library lock failed.".to_owned())?;
            if service.cancellation.is_cancelled() {
                return Err("service_stopped: Reconnect to the current library.".into());
            }
            let mut transfers = service
                .transfers
                .lock()
                .map_err(|_| "service_unavailable: Transfer lock failed.".to_owned())?;
            transfers.expire();
            operation(&library, &mut transfers)
        })
        .await
        .unwrap_or_else(|_| Err("operation_failed: Worker stopped.".into()));
        match result {
            Ok(value) => {
                let mut result =
                    CallToolResult::success(vec![ContentBlock::text(value.to_string())]);
                result.structured_content = Some(value);
                result
            }
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error)]),
        }
    }
}

#[tool_router(router = tool_router)]
impl FullMcp {
    #[tool(
        description = "Lists direct child notes and directories, including archived content. Root is locus://.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn list_entries(&self, Parameters(request): Parameters<Browse>) -> CallToolResult {
        self.run(move |library, _| {
            library.list(
                request.parent.as_deref().unwrap_or("locus://"),
                request.cursor.unwrap_or(0),
                request.limit.unwrap_or(50),
            )
        })
        .await
    }

    #[tool(
        description = "Searches logical addresses, metadata, attachment names and note text including archives, with pagination.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn search_notes(&self, Parameters(request): Parameters<Search>) -> CallToolResult {
        self.run(move |library, _| {
            library.search(
                &request.query,
                request.cursor.unwrap_or(0),
                request.limit.unwrap_or(20),
            )
        })
        .await
    }

    #[tool(
        description = "Reads note or directory metadata and lists all owned resources without server paths.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn resolve_entry(&self, Parameters(request): Parameters<Entry>) -> CallToolResult {
        self.run(move |library, _| library.describe(&request.uri))
            .await
    }

    #[tool(
        description = "Creates an organizational directory in an existing parent; never overwrites.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn create_directory(&self, Parameters(request): Parameters<Entry>) -> CallToolResult {
        self.run(move |library, _| library.create_directory(&request.uri))
            .await
    }

    #[tool(
        description = "Creates a note with raw Markdown and YAML frontmatter in an existing parent or organizational directory; never overwrites.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn create_note(&self, Parameters(request): Parameters<CreateNote>) -> CallToolResult {
        self.run(move |library, _| library.create_note(&request.uri, &request.markdown))
            .await
    }

    #[tool(
        description = "Returns resource size and SHA-256 version for download or conditional mutation.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn stat_content(&self, Parameters(request): Parameters<Target>) -> CallToolResult {
        self.run(move |library, _| library.stat(&request.target))
            .await
    }

    #[tool(
        description = "Writes raw UTF-8 note, canvas or text attachment. expected_version is the read hash or missing for creation.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn write_text(&self, Parameters(request): Parameters<WriteText>) -> CallToolResult {
        self.run(move |library, _| {
            library.write_text(&request.target, &request.text, &request.expected_version)
        })
        .await
    }

    #[tool(
        description = "Replaces exactly one occurrence in raw text, including frontmatter. Rejects ambiguous matches and stale versions.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn patch_text(&self, Parameters(request): Parameters<PatchText>) -> CallToolResult {
        self.run(move |library, _| {
            library.patch_text(
                &request.target,
                &request.expected_version,
                &request.old_text,
                &request.new_text,
            )
        })
        .await
    }

    #[tool(
        description = "Archives or restores a note, preserving unrelated metadata and body.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_archived(&self, Parameters(request): Parameters<Archive>) -> CallToolResult {
        self.run(move |library, _| {
            library.set_archived(&request.uri, &request.expected_version, request.archived)
        })
        .await
    }

    #[tool(
        description = "Permanently deletes exactly one note body, canvas, icon or attachment; leaves the directory and other resources intact.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn delete_content(
        &self,
        Parameters(request): Parameters<DeleteContent>,
    ) -> CallToolResult {
        self.run(move |library, _| {
            library.delete_content(&request.target, &request.expected_version)
        })
        .await
    }

    #[tool(
        description = "Lists all directories and versioned resources for byte-exact subtree export via download_chunk. Recreate parents then notes then resources to import.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn export_manifest(&self, Parameters(request): Parameters<Entry>) -> CallToolResult {
        self.run(move |library, _| library.export_manifest(&request.uri))
            .await
    }

    #[tool(
        description = "Inspects a subtree and returns the content version required for deletion. Rejects unmanaged entries.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn inspect_tree(&self, Parameters(request): Parameters<Entry>) -> CallToolResult {
        self.run(move |library, _| library.inspect_tree(&request.uri))
            .await
    }

    #[tool(
        description = "Permanently deletes a directory. recursive=true explicitly includes all descendant notes, attachments and canvases; requires inspect_tree version.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn delete_entry(&self, Parameters(request): Parameters<DeleteEntry>) -> CallToolResult {
        self.run(move |library, _| {
            library.delete_tree(&request.uri, &request.expected_version, request.recursive)
        })
        .await
    }

    #[tool(
        description = "Previews a note/directory rename or move and affected Markdown links; returns the required move version.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn plan_move(&self, Parameters(request): Parameters<MovePlan>) -> CallToolResult {
        self.run(move |library, _| library.plan_move(&request.uri, &request.destination))
            .await
    }

    #[tool(
        description = "Renames or moves a complete note/directory subtree and updates resolvable Markdown links using a plan_move version. Destination must not exist.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn move_entry(&self, Parameters(request): Parameters<MoveEntry>) -> CallToolResult {
        self.run(move |library, _| {
            library.move_tree(
                &request.uri,
                &request.destination,
                &request.expected_version,
            )
        })
        .await
    }

    #[tool(
        description = "Renames or moves an attachment without overwriting and updates recognized Markdown links. Returns affected notes; review canvas/script references explicitly.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn move_attachment(
        &self,
        Parameters(request): Parameters<MoveAttachment>,
    ) -> CallToolResult {
        self.run(move |library, _| {
            library.move_attachment(
                &request.source,
                &request.destination,
                &request.expected_version,
            )
        })
        .await
    }

    #[tool(
        description = "Starts a staged upload, at most 1 GiB. Optional sha256 verifies bytes before commit. No destination changes until finish_upload.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn begin_upload(&self, Parameters(request): Parameters<BeginUpload>) -> CallToolResult {
        self.run(move |library, transfers| {
            transfers.begin(
                library,
                request.target,
                request.size,
                request.sha256,
                request.expected_version,
            )
        })
        .await
    }

    #[tool(
        description = "Appends up to 256 KiB decoded base64 bytes at the specified offset. Exact retries are accepted. Uploads expire after 10 idle minutes.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn upload_chunk(&self, Parameters(request): Parameters<UploadChunk>) -> CallToolResult {
        self.run(move |_, transfers| {
            transfers.append(&request.upload_id, request.offset, &request.base64)
        })
        .await
    }

    #[tool(
        description = "Checks total size and optional SHA-256 then atomically commits uploaded content with version checking. Icon uploads use image conversion.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn finish_upload(&self, Parameters(request): Parameters<UploadId>) -> CallToolResult {
        self.run(move |library, transfers| transfers.finish(library, &request.upload_id))
            .await
    }

    #[tool(
        description = "Discards a pending upload without changing the target content.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn cancel_upload(&self, Parameters(request): Parameters<UploadId>) -> CallToolResult {
        self.run(move |_, transfers| transfers.cancel(&request.upload_id))
            .await
    }

    #[tool(
        description = "Reads raw UTF-8 text including frontmatter, with a zero-based line offset and limit (default 200, maximum 2000). Returns a full-content version, totalLines and nextLine. Files over 8 MiB use download_chunk.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn read_text(&self, Parameters(request): Parameters<ReadText>) -> CallToolResult {
        self.run(move |library, _| {
            let limit = request.limit.unwrap_or(200);
            if limit == 0 || limit > 2000 { return Err("invalid_limit: limit must be 1..2000.".into()); }
            let (content, version) = library.read_text(&request.target)?;
            let lines = content.split_inclusive('\n').collect::<Vec<_>>();
            let start = request.start_line.unwrap_or(0);
            if start > lines.len() { return Err("invalid_offset: Line offset exceeds content.".into()); }
            let end = start.saturating_add(limit).min(lines.len());
            Ok(json!({"target": request.target, "text": lines[start..end].concat(), "version": version, "totalLines": lines.len(), "nextLine": (end < lines.len()).then_some(end)}))
        }).await
    }

    #[tool(
        description = "Downloads at most 256 KiB of a resource as base64 bytes. Obtain version and size from stat_content; repeat with nextOffset until eof. Rejects changed content.",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn download_chunk(
        &self,
        Parameters(request): Parameters<DownloadChunk>,
    ) -> CallToolResult {
        self.run(move |library, _| {
            use base64::Engine;
            let metadata = library.stat(&request.target)?;
            let bytes = library.read_chunk(&request.target, &request.version, request.offset, request.limit.unwrap_or(64 * 1024))?;
            let next = request.offset + bytes.len() as u64;
            Ok(json!({"target": request.target, "version": request.version, "base64": base64::engine::general_purpose::STANDARD.encode(bytes), "nextOffset": next, "eof": next == metadata["size"].as_u64().unwrap_or(0)}))
        }).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for FullMcp {
    async fn list_resources(
        &self,
        request: Option<rmcp::model::PaginatedRequestParams>,
        _: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListResourcesResult, rmcp::ErrorData> {
        crate::docs::list(request)
    }

    async fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParams,
        _: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ReadResourceResponse, rmcp::ErrorData> {
        crate::docs::read(&request.uri)
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("locus-full", env!("CARGO_PKG_VERSION")))
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
        .with_instructions(format!("{INSTRUCTIONS} {}", crate::docs::INSTRUCTIONS))
    }
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![ProtocolVersion::V_2025_11_25])
    }
}
