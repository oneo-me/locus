# MCP access

Enable **Settings → MCP → Enable read-only discovery** after selecting a library. The default Streamable HTTP endpoint is:

```text
http://127.0.0.1:2301/mcp
```

The port is configurable; choose Apply connection after editing it. Disabling the service or changing libraries closes the old listener. Settings displays the real listening address or error. The transport is stateless; clients must not depend on durable server sessions.

The three tools are `list_articles`, `search_articles`, and `resolve_article`. They return note metadata and absolute local file/directory paths. Clients read and edit those paths with their own filesystem tools and therefore need access to the same files. MCP does not transfer complete content, write notes, or run Agents.

Local-network mode listens beyond loopback without authentication. Connected clients can search note content and obtain paths. Use it only on a trusted network; change the client URL to this computer's IP. Host and Origin checks defend against unrelated browser origins and arbitrary hostnames, not unauthorized clients already able to reach the trusted endpoint.

## Install authoring guidance

In Settings → Agents, choose **Install in library** or **Install globally**:

- Library copies the bundled skill to `<library>/.agents/skills/locus`.
- Global copies it to `~/.agents/skills/locus`.

Existing files, directories, and symlinks at the destination are not overwritten. Move the old installation before reinstalling. Redirected parent directories are rejected. Installation failures report their reason; an incomplete new installation may require inspection before retrying. The app does not remove a copy at another location or start an Agent automatically.

The bundled [skill](../source/resources/skills/locus/SKILL.md) defines note workflows, with [attachment](../source/resources/skills/locus/references/attachments.md) and [canvas](../source/resources/skills/locus/references/canvas.md) references. Copies remain usable when the app is moved, but do not update automatically. Client discovery and reloading depend on the client's own skill support.
