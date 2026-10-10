package main

import (
	"os"
	"strings"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/skills"
	"github.com/oneo-me/onevra/components"
	"github.com/oneo-me/onevra/shells/sidebar"
)

func (a *application) selectSetting(c *ui.Context, value *string, options []string) {
	control := components.Select(c, value)
	control.Trigger.Children(func() { components.Text(c, a.text(*value)).SingleLine() })
	control.Popup(func(ui.Element) {
		for _, option := range options {
			control.Item(option).Children(func() { components.Text(c, a.text(option)).SingleLine() })
		}
	})
	if control.Trigger.Changed() {
		a.saveConfig()
		a.installMenu()
		a.syncCanvasTheme()
	}
}
func (a *application) settings(c *ui.Context) {
	item := func(id, title string, build func(*ui.Context)) components.SettingsItem {
		return components.SettingsItem{ID: id, Title: a.text(title), Build: build}
	}
	categories := []components.SettingsCategory{
		{ID: "general", Title: a.text("General"), Groups: []components.SettingsGroup{{ID: "library", Items: []components.SettingsItem{
			item("library", "Library", func(c *ui.Context) {
				components.Text(c, a.model.Root).Wrap()
				components.Button(c, a.text("Open library"), components.ButtonSecondary).OnClick(a.chooseLibrary)
			}),
			item("editor", "Editor", func(c *ui.Context) { a.selectSetting(c, &a.cfg.Editor, []string{"system", "visual-studio-code"}) }),
			item("export", "Export directory", func(c *ui.Context) {
				if components.TextBox(c, &a.cfg.Export, components.TextBoxOptions{}).Placeholder("~/Downloads").Changed() {
					a.saveConfig()
				}
			}),
		}}}},
		{ID: "archive", Title: a.text("Archived notes"), Groups: []components.SettingsGroup{{ID: "archived", Items: []components.SettingsItem{
			item("restore", "Restore", func(c *ui.Context) {
				for _, note := range a.archived {
					components.Button(c, note.Title, components.ButtonSecondary).OnClick(func() { a.restoreNote(note.Path) })
				}
			}),
		}}}},
		{ID: "appearance", Title: a.text("Appearance"), Groups: []components.SettingsGroup{{ID: "appearance", Items: []components.SettingsItem{
			item("theme", "Theme", func(c *ui.Context) { a.selectSetting(c, &a.cfg.Theme, []string{"system", "light", "dark"}) }),
			item("language", "Language", func(c *ui.Context) { a.selectSetting(c, &a.cfg.Language, []string{"system", "english", "chinese"}) }),
		}}}},
		{ID: "agents", Title: a.text("Agents"), Groups: []components.SettingsGroup{{ID: "terminal", Items: []components.SettingsItem{
			item("terminal", "Terminal", func(c *ui.Context) { a.selectSetting(c, &a.cfg.Terminal, []string{"system", "ghostty"}) }),
			item("agent", "Agent", func(c *ui.Context) {
				a.selectSetting(c, &a.cfg.Agent, []string{"codex", "pi", "opencode", "claude-code"})
			}),
			item("shell", "Shell", func(c *ui.Context) { a.selectSetting(c, &a.cfg.Shell, []string{"default", "zsh", "bash", "fish"}) }),
			item("launch", "Open Agent", func(c *ui.Context) {
				components.Button(c, a.text("Open Agent"), components.ButtonSecondary).Disabled(a.model.Root == "" || a.launching).OnClick(a.launchAgent)
			}),
			item("skills", "Install authoring skill", func(c *ui.Context) {
				components.Button(c, a.text("Install in library"), components.ButtonSecondary).Disabled(a.model.Root == "").OnClick(func() { a.installSkill(a.model.Root) })
				components.Button(c, a.text("Install globally"), components.ButtonSecondary).OnClick(func() {
					home, err := os.UserHomeDir()
					if err != nil {
						a.feedback = err.Error()
					} else {
						a.installSkill(home)
					}
				})
			}),
		}}}},
		{ID: "mcp", Title: "MCP", Groups: []components.SettingsGroup{{ID: "connection", Items: []components.SettingsItem{
			item("enable", "Enable read-only discovery", func(c *ui.Context) {
				if components.Switch(c, &a.cfg.MCP, a.text("Enable read-only discovery")).Changed() {
					a.saveConfig()
					a.restartMCP()
				}
			}),
			{ID: "network", Title: a.text("Local network"), Description: a.text("Network clients can search note content and obtain local paths without authentication."), Build: func(c *ui.Context) {
				if components.Switch(c, &a.cfg.Network, a.text("Local network")).Changed() {
					a.saveConfig()
					a.restartMCP()
				}
			}},
			item("port", "Port", func(c *ui.Context) {
				port := float64(a.cfg.Port)
				field := components.NumberInput(c, &port, 1, 65535, 1)
				if field.Changed() {
					a.cfg.Port = int(port)
					a.saveConfig()
				}
				components.Button(c, a.text("Apply connection"), components.ButtonSecondary).OnClick(a.restartMCP)
			}),
			item("status", "Status", func(c *ui.Context) {
				status := a.text("Stopped")
				if a.service != nil {
					status = a.service.Address + "/mcp"
				}
				if a.mcpError != "" {
					status = a.mcpError
				}
				components.Text(c, status).Wrap()
			}),
		}}}},
		{ID: "about", Title: a.text("About"), Groups: []components.SettingsGroup{{ID: "application", Items: []components.SettingsItem{
			{ID: "version", Title: "Locus 0.2026.1", Description: "Go · Onevra · MyGo · MIT"},
			{ID: "updates", Title: a.text("Updates"), Description: a.text("Automatic updates are not configured. Download releases from GitHub."), Build: func(c *ui.Context) {
				components.Button(c, "GitHub Releases", components.ButtonSecondary).OnClick(func() { a.openLink("https://github.com/oneo-me/locus/releases/latest") })
			}},
		}}}},
	}
	ui.Column(c).Fill().MinHeight(0).Padding(sidebar.HeaderHeight(c), 16, 16, 16).Children(func() {
		components.Button(c, a.text("Back"), components.ButtonGhost).OnClick(a.toggleSettings)
		for _, message := range []string{a.storageError, a.feedback} {
			if message != "" {
				components.Text(c, message).Wrap().TextColor(c.Theme().Danger)
			}
		}
		components.Settings(c, &a.settingState, categories, components.SettingsOptions{Title: a.text("Settings")}).Fill()
	})
}
func (a *application) installSkill(root string) {
	go func() {
		err := skills.Install(root)
		a.post(func() {
			if err != nil {
				a.feedback = err.Error()
			} else {
				a.feedback = a.text("Skill installed")
			}
		})
	}()
}
func (a *application) text(english string) string {
	chinese := a.cfg.Language == "chinese" || (a.cfg.Language != "english" && strings.HasPrefix(strings.ToLower(mygo.App.Locale()), "zh"))
	if chinese {
		if value, ok := zh[english]; ok {
			return value
		}
	}
	return english
}

var zh = map[string]string{
	"Archive note": "归档笔记", "Archive": "归档", "Archived notes": "已归档笔记", "Restore": "恢复", "Cancel": "取消", "Hide this note and its descendants? Files are preserved and can be restored in Settings.": "隐藏这篇笔记及其子笔记？文件会保留，可在设置中恢复。",
	"Open library": "打开文档库", "Search notes": "搜索笔记", "Settings": "设置", "Refresh": "刷新", "Open Agent": "打开 Agent", "Loading…": "加载中…", "Back": "返回", "Forward": "前进", "Copy reference": "复制引用", "Edit note": "编辑笔记", "Open directory": "打开所在目录", "Export canvas": "导出画布", "Exported": "已导出", "File": "文件", "View": "视图", "General": "通用", "Library": "文档库", "Editor": "编辑器", "Export directory": "导出目录", "Appearance": "外观", "Theme": "主题", "Language": "语言", "Agents": "Agent", "Terminal": "终端", "Shell": "Shell", "Install authoring skill": "安装编写技能", "Install in library": "安装到文档库", "Install globally": "安装到全局", "Skill installed": "技能已安装", "Enable read-only discovery": "启用只读发现", "Local network": "局域网", "Port": "端口", "Apply connection": "应用连接设置", "Status": "状态", "Stopped": "已停止", "About": "关于", "Updates": "更新", "Automatic updates are not configured. Download releases from GitHub.": "尚未配置自动更新，请从 GitHub 下载发布版本。", "Network clients can search note content and obtain local paths without authentication.": "网络客户端无需身份验证即可搜索笔记内容并获取本机文件路径。", "system": "跟随系统", "default": "默认", "light": "浅色", "dark": "深色", "english": "English", "chinese": "简体中文", "visual-studio-code": "VS Code", "Select a note in the sidebar.": "在侧栏选择一篇笔记。", "Open a Markdown library to start reading.": "打开 Markdown 文档库开始阅读。", "No visible notes. Notes use a directory containing index.md.": "没有可见笔记。每篇笔记是包含 index.md 的目录。",
}
