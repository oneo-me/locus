export const repositoryUrl = "https://github.com/oneo-me/locus";
// GitHub redirects to the newest published release, or to the release list when none exists.
export const downloadUrl = `${repositoryUrl}/releases/latest`;
export const releasesUrl = `${repositoryUrl}/releases`;

export type Locale = "en" | "zh-CN" | "zh-TW" | "ja" | "ko";

export const localePaths: Record<Locale, string> = {
  en: "/",
  "zh-CN": "/zh-CN",
  "zh-TW": "/zh-TW",
  ja: "/ja",
  ko: "/ko",
};

export const locales = Object.keys(localePaths) as Locale[];

export type Content = {
  title: string;
  description: string;
  languageName: string;
  languageLabel: string;
  themeLabel: string;
  themeLight: string;
  themeDark: string;
  themeSystem: string;
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
    languageLabel: "Language",
    themeLabel: "Theme",
    themeLight: "Light",
    themeDark: "Dark",
    themeSystem: "System",
    badge: "Open source · MIT",
    heading: "Your notes, as plain files.",
    lead: "Locus is a local-first Markdown workspace with a native desktop interface. Read and organize notes in Locus, edit them in your favorite editor, and let Agents discover the same files.",
    download: "Download Locus",
    platform: "Apple Silicon and Intel macOS · Windows x64 and ARM64 · Linux x64 and ARM64",
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
    languageLabel: "语言",
    themeLabel: "主题",
    themeLight: "浅色",
    themeDark: "深色",
    themeSystem: "跟随系统",
    badge: "开源 · MIT",
    heading: "你的笔记，就是普通文件。",
    lead: "Locus 是一个本地优先、拥有原生桌面界面的 Markdown 工作区。在 Locus 中阅读和整理笔记，用你喜欢的编辑器编辑，并让 Agent 发现同一批文件。",
    download: "下载 Locus",
    platform: "Apple Silicon 与 Intel macOS · Windows x64 与 ARM64 · Linux x64 与 ARM64",
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
  "zh-TW": {
    title: "Locus — 本地優先的 Markdown 工作區",
    description:
      "原生桌面 Markdown 工作區：筆記始終是普通檔案，支援 HTML 畫布，並透過唯讀 MCP 供 Agent 發現。",
    languageName: "繁體中文",
    languageLabel: "語言",
    themeLabel: "主題",
    themeLight: "淺色",
    themeDark: "深色",
    themeSystem: "跟隨系統",
    badge: "開源 · MIT",
    heading: "你的筆記，就是普通檔案。",
    lead: "Locus 是一個本地優先、擁有原生桌面介面的 Markdown 工作區。在 Locus 中閱讀和整理筆記，用你喜歡的編輯器編輯，並讓 Agent 發現同一批檔案。",
    download: "下載 Locus",
    platform: "Apple Silicon 與 Intel macOS · Windows x64 與 ARM64 · Linux x64 與 ARM64",
    source: "在 GitHub 上查看",
    allReleases: "全部版本",
    featuresHeading: "圍繞你的檔案構建",
    features: [
      {
        title: "純 Markdown",
        body: "每篇筆記都是一個包含 index.md、附件和子筆記的目錄，內容不被鎖在資料庫裡。",
      },
      {
        title: "畫布",
        body: "筆記可以擁有 HTML 畫布，透過唯讀資源協定在工作區內顯示。",
      },
      {
        title: "Agent 發現",
        body: "可選的唯讀 MCP 讓 Agent 搜尋和定位筆記，編輯仍由它們自己的工具完成。",
      },
      {
        title: "內建終端機",
        body: "僅在你需要時，從標題列彈出視窗開啟 Shell 或 Agent CLI。",
      },
    ],
    license: "基於 MIT 授權發布。",
  },
  ja: {
    title: "Locus — ローカルファーストの Markdown ワークスペース",
    description:
      "ノートが普通のファイルのまま残る、ネイティブのデスクトップ Markdown ワークスペース。HTML キャンバスと、Agent 向けの読み取り専用 MCP に対応します。",
    languageName: "日本語",
    languageLabel: "言語",
    themeLabel: "テーマ",
    themeLight: "ライト",
    themeDark: "ダーク",
    themeSystem: "システム",
    badge: "オープンソース · MIT",
    heading: "ノートは、普通のファイルです。",
    lead: "Locus は、ネイティブのデスクトップ画面を持つローカルファーストの Markdown ワークスペースです。Locus でノートを読み、整理し、好きなエディタで編集し、同じファイルを Agent に見つけさせます。",
    download: "Locus をダウンロード",
    platform: "Apple Silicon と Intel の macOS · Windows x64 と ARM64 · Linux x64 と ARM64",
    source: "GitHub で見る",
    allReleases: "すべてのリリース",
    featuresHeading: "ファイルを中心に",
    features: [
      {
        title: "プレーンな Markdown",
        body: "各ノートは index.md、添付ファイル、子ノートを含むディレクトリです。データベースには閉じ込めません。",
      },
      {
        title: "キャンバス",
        body: "ノートは HTML キャンバスを持てます。読み取り専用のリソースプロトコルでワークスペース内に表示します。",
      },
      {
        title: "Agent による検出",
        body: "任意の読み取り専用 MCP で Agent がノートを検索、特定できます。編集は Agent 自身のツールが行います。",
      },
      {
        title: "内蔵ターミナル",
        body: "必要なときだけ、タイトルバーのポップオーバーからシェルまたは Agent CLI を開きます。",
      },
    ],
    license: "MIT ライセンスで公開しています。",
  },
  ko: {
    title: "Locus — 로컬 우선 Markdown 작업 공간",
    description:
      "노트가 평범한 파일로 남는 네이티브 데스크톱 Markdown 작업 공간입니다. HTML 캔버스와 Agent용 읽기 전용 MCP를 제공합니다.",
    languageName: "한국어",
    languageLabel: "언어",
    themeLabel: "테마",
    themeLight: "라이트",
    themeDark: "다크",
    themeSystem: "시스템",
    badge: "오픈 소스 · MIT",
    heading: "노트는 평범한 파일입니다.",
    lead: "Locus는 네이티브 데스크톱 화면을 갖춘 로컬 우선 Markdown 작업 공간입니다. Locus에서 노트를 읽고 정리하고, 원하는 편집기에서 수정하고, Agent가 같은 파일을 찾게 하세요.",
    download: "Locus 다운로드",
    platform: "Apple Silicon 및 Intel macOS · Windows x64 및 ARM64 · Linux x64 및 ARM64",
    source: "GitHub에서 보기",
    allReleases: "모든 릴리스",
    featuresHeading: "파일을 중심으로",
    features: [
      {
        title: "플레인 Markdown",
        body: "각 노트는 index.md, 첨부 파일, 하위 노트가 있는 디렉터리입니다. 데이터베이스에 잠기지 않습니다.",
      },
      {
        title: "캔버스",
        body: "노트는 HTML 캔버스를 가질 수 있으며, 읽기 전용 리소스 프로토콜로 작업 공간 안에 표시합니다.",
      },
      {
        title: "Agent 탐색",
        body: "선택적 읽기 전용 MCP로 Agent가 노트를 검색하고 찾습니다. 편집은 Agent 자신의 도구가 합니다.",
      },
      {
        title: "내장 터미널",
        body: "필요할 때만 제목 표시줄 팝오버에서 셸 또는 Agent CLI를 엽니다.",
      },
    ],
    license: "MIT 라이선스로 배포합니다.",
  },
};
