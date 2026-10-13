export const repositoryUrl = "https://github.com/oneo-me/locus";
// GitHub redirects to the newest published release, or to the release list when none exists.
export const downloadUrl = `${repositoryUrl}/releases/latest`;
export const releasesUrl = `${repositoryUrl}/releases`;

export type Locale = "en" | "zh-CN";

export const localePaths: Record<Locale, string> = {
  en: "/",
  "zh-CN": "/zh-CN",
};

export type Content = {
  title: string;
  description: string;
  languageName: string;
  badge: string;
  heading: string;
  lead: string;
  download: string;
  platform: string;
  source: string;
  allReleases: string;
  featuresHeading: string;
  features: { title: string; body: string }[];
  license: string;
};

export const content: Record<Locale, Content> = {
  en: {
    title: "Locus — local-first Markdown workspace",
    description:
      "A native desktop workspace for Markdown notes that stay ordinary files, with HTML canvases and read-only MCP discovery for Agents.",
    languageName: "English",
    badge: "Open source · MIT",
    heading: "Your notes, as plain files.",
    lead: "Locus is a local-first Markdown workspace with a native desktop interface. Read and organize notes in Locus, edit them in your favorite editor, and let Agents discover the same files.",
    download: "Download Locus",
    platform: "Apple Silicon and Intel macOS · Windows x64 · Linux x64",
    source: "View on GitHub",
    allReleases: "All releases",
    featuresHeading: "Built around your files",
    features: [
      {
        title: "Plain Markdown",
        body: "Each note is a directory with index.md, attachments, and child notes. Nothing is locked in a database.",
      },
      {
        title: "Canvases",
        body: "Notes can own HTML canvases that render inside the workspace through a read-only resource protocol.",
      },
      {
        title: "Agent discovery",
        body: "Optional read-only MCP lets Agents search and locate notes, while edits stay in their own tools.",
      },
      {
        title: "Built-in terminal",
        body: "Open your shell or Agent CLI in a titlebar popover, only when you ask for it.",
      },
    ],
    license: "Released under the MIT License.",
  },
  "zh-CN": {
    title: "Locus — 本地优先的 Markdown 工作区",
    description:
      "原生桌面 Markdown 工作区：笔记始终是普通文件，支持 HTML 画布，并通过只读 MCP 供 Agent 发现。",
    languageName: "简体中文",
    badge: "开源 · MIT",
    heading: "你的笔记，就是普通文件。",
    lead: "Locus 是一个本地优先、拥有原生桌面界面的 Markdown 工作区。在 Locus 中阅读和整理笔记，用你喜欢的编辑器编辑，并让 Agent 发现同一批文件。",
    download: "下载 Locus",
    platform: "Apple Silicon 与 Intel macOS · Windows x64 · Linux x64",
    source: "在 GitHub 上查看",
    allReleases: "全部版本",
    featuresHeading: "围绕你的文件构建",
    features: [
      {
        title: "纯 Markdown",
        body: "每篇笔记都是一个包含 index.md、附件和子笔记的目录，内容不被锁在数据库里。",
      },
      {
        title: "画布",
        body: "笔记可以拥有 HTML 画布，通过只读资源协议在工作区内显示。",
      },
      {
        title: "Agent 发现",
        body: "可选的只读 MCP 让 Agent 搜索和定位笔记，编辑仍由它们自己的工具完成。",
      },
      {
        title: "内置终端",
        body: "仅在你需要时，从标题栏弹出窗口打开 Shell 或 Agent CLI。",
      },
    ],
    license: "基于 MIT 许可证发布。",
  },
};
