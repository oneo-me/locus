# MCP access

Locus provides read-only note discovery at the Streamable HTTP URL `http://127.0.0.1:2301/mcp`. Enable it in Settings → MCP; the service is disabled by default, and its port is configurable. Configure the client with this URL, replacing the port when changed.

The three tools are `list_articles`, `search_articles` and `resolve_article`. They return note metadata and absolute local file paths. Clients use their own filesystem tools to read and edit content, so they need access to the selected library's files. The service does not provide a full endpoint, content-write or transfer tools, or built-in documentation resources.

Local-network mode exposes discovery without authentication: connected clients can search note content and obtain local paths. For network access, replace the host with this computer's IP. Host validation accepts localhost, loopback addresses and the local IP receiving the connection; arbitrary hostnames are rejected. Changing the library or restarting the service invalidates existing sessions. Settings and the sidebar show runtime status; settings exposes startup errors.

## Install authoring skills

In Settings → Skills, select Library or Global under Installation location to install automatically:

- Library installs to `<library>/.agents/skills/locus`; select a library in General first.
- Global installs to `~/.agents/skills/locus`.

Installation reports the actual destination or a failure. Installation creates a directory symlink to the running application’s bundled `skills/locus` directory. Select the location again to update an existing link or retry. Real directories/files and other skills are preserved; move an existing real installation before retrying. Changing the selection installs the new link first, then removes the Locus symlink from the other location. If installation fails, the old link remains; cleanup errors are reported separately. Real installations at the old location are preserved. Moving or deleting the application can break the link; select the location again from the new application location. Windows may require Developer Mode or permission to create directory symlinks; failures are reported rather than falling back to copying. Agent discovery/reloading follows the client's own skill support.

The [Locus skill](../source/resources/skills/locus/SKILL.md) contains note discovery and authoring workflows, with task-specific [attachment](../source/resources/skills/locus/references/attachments.md) and [canvas](../source/resources/skills/locus/references/canvas.md) references. These files are packaged under `Contents/Resources/skills` on macOS and `resources/skills` in the Windows portable directory; installation links to that resource directory. Debug runs outside an application bundle link to the source resources. MCP descriptions remain minimal.
