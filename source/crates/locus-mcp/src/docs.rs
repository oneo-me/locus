use rmcp::ErrorData;
use rmcp::model::{
    ListResourcesResult, PaginatedRequestParams, ReadResourceResponse, ReadResourceResult,
    Resource, ResourceContents,
};

pub(super) const INSTRUCTIONS: &str = "Before authoring attachments or canvases, use resources/read to read locus-docs://attachments or locus-docs://canvas. Discover both with resources/list. These read-only English software specifications are built into Locus, independent of the selected library. They are MCP resource URIs, not note addresses; do not pass them to article or library tools.";

struct Document {
    uri: &'static str,
    name: &'static str,
    description: &'static str,
    text: &'static str,
}

const DOCUMENTS: &[Document] = &[
    Document {
        uri: "locus-docs://attachments",
        name: "Attachments",
        description: "English specification for note attachments, galleries, and file references.",
        text: include_str!("../../../resources/docs/attachments.md"),
    },
    Document {
        uri: "locus-docs://canvas",
        name: "Canvas authoring",
        description: "English specification for canvas fragments, resources, lifecycle, and templates.",
        text: include_str!("../../../resources/docs/canvas.md"),
    },
];

pub(super) fn list(
    request: Option<PaginatedRequestParams>,
) -> Result<ListResourcesResult, ErrorData> {
    if request.is_some_and(|request| request.cursor.is_some()) {
        return Err(ErrorData::invalid_params("Unknown resource cursor", None));
    }
    let resources = DOCUMENTS
        .iter()
        .map(|document| {
            Resource::new(document.uri, document.name)
                .with_description(document.description)
                .with_mime_type("text/markdown")
                .with_size(document.text.len() as u64)
        })
        .collect();
    Ok(ListResourcesResult {
        resources,
        ..Default::default()
    })
}

pub(super) fn read(uri: &str) -> Result<ReadResourceResponse, ErrorData> {
    let document = DOCUMENTS
        .iter()
        .find(|document| document.uri == uri)
        .ok_or_else(|| {
            ErrorData::resource_not_found("Unknown Locus documentation resource", None)
        })?;
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(document.text, document.uri).with_mime_type("text/markdown"),
    ])
    .into())
}
