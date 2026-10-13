package main

import (
	"os"
	"strings"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/skills"
	launch "github.com/oneo-me/locus/internal/terminal"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/icons/lucide"
	"github.com/oneo-me/onevra/source/shells/sidebar"
)

func (a *application) selectSetting(c *ui.Context, label string, value *string, options []string) {
	control := components.Select(c, value)
	control.Trigger.Label(a.text(label)).Width(c.Theme().Space(40)).Shrink(0).Children(func() { components.Text(c, a.text(*value)).SingleLine() })
	control.Popup(func(ui.Element) {
		for _, option := range options {
			control.Item(option).Label(a.text(option)).Children(func() { components.Text(c, a.text(option)).SingleLine() })
		}
	})
	if control.Trigger.Changed() {
		a.saveConfig()
		a.installMenu()
		a.syncCanvasTheme()
	}
}
func (a *application) settingsDialog(c *ui.Context) {
	if !a.settingsOpen {
		return
	}
	w, h := c.Size()
	inset := sidebar.HeaderHeight(c)
	width := min(c.Theme().Space(210), w-2*inset)
	components.Dialog(c, &a.settingsOpen, components.DialogOptions{Label: a.text("Settings"), Inset: inset}, func(panel ui.Element) {
		panel.Width(width).Height(min(c.Theme().Space(145), h-2*inset)).Padding(0).Gap(0).Clip()
		a.settings(c, width < c.Theme().Space(170))
	})
}

func (a *application) settings(c *ui.Context, narrow bool) {
	item := func(id, title string, build func(*ui.Context)) components.SettingsItem {
		return components.SettingsItem{ID: id, Title: a.text(title), Build: build}
	}
	categories := []components.SettingsCategory{
		{ID: "general", Section: a.text("Application"), Icon: lucide.Settings(), Title: a.text("General"), Groups: []components.SettingsGroup{{ID: "library", Items: []components.SettingsItem{
			{ID: "library", Title: a.text("Library"), Description: a.libraryDescription(), Build: func(c *ui.Context) {
				components.Button(c, a.text("Select library"), components.ButtonSecondary).Shrink(0).Disabled(a.choosing).OnClick(a.chooseLibrary)
			}},
			item("editor", "Editor", func(c *ui.Context) {
				a.selectSetting(c, "Editor", &a.cfg.Editor, []string{"system", "visual-studio-code"})
			}),
			item("export", "Export directory", func(c *ui.Context) {
				if components.TextBox(c, &a.cfg.Export, components.TextBoxOptions{}).Label(a.text("Export directory")).Width(c.Theme().Space(48)).Shrink(0).Placeholder("~/Downloads").Changed() {
					a.saveConfig()
				}
			}),
		}}}},
		{ID: "appearance", Section: a.text("Application"), Icon: lucide.Palette(), Title: a.text("Appearance"), Groups: []components.SettingsGroup{{ID: "appearance", Items: []components.SettingsItem{
			item("theme", "Theme", func(c *ui.Context) { a.selectSetting(c, "Theme", &a.cfg.Theme, []string{"system", "light", "dark"}) }),
			item("language", "Language", func(c *ui.Context) {
				a.selectSetting(c, "Language", &a.cfg.Language, []string{"system", "english", "chinese"})
			}),
		}}}},
		{ID: "agents", Section: a.text("Integrations"), Icon: lucide.Bot(), Title: a.text("Agents"), Groups: []components.SettingsGroup{{ID: "terminal", Items: []components.SettingsItem{
			item("shell", "Shell", func(c *ui.Context) {
				before := a.cfg.Shell
				a.selectSetting(c, "Shell", &a.cfg.Shell, launch.Shells())
				if before != a.cfg.Shell {
					a.terminalPreferencesChanged = true
				}
			}),
			item("agent", "Agent", func(c *ui.Context) {
				before := a.cfg.Agent
				a.selectSetting(c, "Agent", &a.cfg.Agent, append([]string{"none"}, launch.Agents()...))
				if before != a.cfg.Agent {
					a.terminalPreferencesChanged = true
				}
			}),
			item("library-skill", "Install authoring skill", func(c *ui.Context) {
				components.Button(c, a.text("Install in library"), components.ButtonSecondary).Disabled(a.model.Root == "").OnClick(func() { a.installSkill(a.model.Root) })
			}),
			item("global-skill", "Install authoring skill globally", func(c *ui.Context) {
				components.Button(c, a.text("Install globally"), components.ButtonSecondary).OnClick(func() {
					home, err := os.UserHomeDir()
					if err != nil {
						a.errorNotice(err.Error())
					} else {
						a.installSkill(home)
					}
				})
			}),
		}}}},
		{ID: "mcp", Section: a.text("Integrations"), Icon: lucide.Radio(), Title: "MCP", Groups: []components.SettingsGroup{{ID: "connection", Items: []components.SettingsItem{
			item("enable", "Enable read-only discovery", func(c *ui.Context) {
				if components.Switch(c, &a.cfg.MCP, a.text("Enable read-only discovery"), components.SwitchOptions{HideLabel: true}).Changed() {
					a.saveConfig()
					a.restartMCP()
				}
			}),
			{ID: "network", Title: a.text("Local network"), Description: a.text("Network clients can search note content and obtain local paths without authentication."), Build: func(c *ui.Context) {
				if components.Switch(c, &a.cfg.Network, a.text("Local network"), components.SwitchOptions{HideLabel: true}).Changed() {
					a.saveConfig()
					a.restartMCP()
				}
			}},
			item("port", "Port", func(c *ui.Context) {
				port := float64(a.cfg.Port)
				field := components.NumberInput(c, &port, 1, 65535, 1).Label(a.text("Port")).Width(c.Theme().Space(40)).Shrink(0)
				if field.Changed() {
					a.cfg.Port = int(port)
					a.saveConfig()
				}
			}),
			item("apply", "Apply connection", func(c *ui.Context) {
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
				components.Text(c, status).Width(c.Theme().Space(48)).Shrink(0).Wrap()
			}),
		}}}},
	}
	ui.Column(c).Fill().MinHeight(0).Children(func() {
		components.Settings(c, &a.settingState, categories, components.SettingsOptions{
			Title: a.text("Settings"), Narrow: narrow,
			SearchLabel: a.text("Search settings"), SearchPlaceholder: a.text("Search settings…"),
			CategoryLabel: a.text("Settings category"), ResultsTitle: a.text("Search results"), EmptyText: a.text("No matching settings."),
			HeaderActions: func(c *ui.Context) {
				sidebar.IconButton(c, lucide.X(), a.text("Close settings")).OnClick(a.toggleSettings)
			},
		}).Grow(1).MinHeight(0)
	})
}
func (a *application) libraryDescription() string {
	if a.model.Root == "" {
		return a.text("Choose a library")
	}
	return a.model.Root
}

func (a *application) installSkill(root string) {
	go func() {
		resources, err := mygo.App.Path(mygo.PathResources)
		if err == nil {
			err = skills.Install(root, resources)
		}
		a.post(func() {
			if err != nil {
				a.errorNotice(err.Error())
			} else {
				a.successNotice(a.text("Skill installed"))
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
	"Terminal panel": "终端面板", "none": "无（仅 Shell）",
	"Application": "应用", "Integrations": "集成", "Select library": "选择文档库",
	"Choose a library": "选择笔记库", "Close settings": "关闭设置",
	"Search settings": "搜索设置", "Search settings…": "搜索设置…", "Settings category": "设置分类", "Search results": "搜索结果", "No matching settings.": "没有匹配的设置。", "Install authoring skill globally": "全局安装编写技能",
	"More actions":       "更多操作",
	"Upload icon":        "上传图标",
	"Icon upload failed": "上传图标失败",
	"Icon updated":       "图标已更新",
	"Include only local images with alt text in the gallery.": "图库中只能包含带有替代文本的本地图片。",
	"Include at least one image in the gallery.":              "请在图库中至少添加一张图片。",
	"Image unavailable": "图片不可用", "Close": "关闭",
	"Could not show attachments": "无法显示附件", "Use only assets/filename links with matching filenames.": "请仅填写 assets/文件名 链接，链接文字须与文件名一致。",
	"Open library": "打开文档库", "Search notes": "搜索笔记", "Settings": "设置", "Refresh": "刷新", "Open Agent": "打开 Agent", "Loading…": "加载中…", "Back": "返回", "Forward": "前进", "Copy reference": "复制引用", "Edit note": "编辑笔记", "Open directory": "打开所在目录", "Export canvas": "导出画布", "Exported": "已导出", "File": "文件", "View": "视图", "General": "通用", "Library": "文档库", "Editor": "编辑器", "Export directory": "导出目录", "Appearance": "外观", "Theme": "主题", "Language": "语言", "Agents": "Agent", "Terminal": "终端", "Shell": "Shell", "Install authoring skill": "安装编写技能", "Install in library": "安装到文档库", "Install globally": "安装到全局", "Skill installed": "技能已安装", "Enable read-only discovery": "启用只读发现", "Local network": "局域网", "Port": "端口", "Apply connection": "应用连接设置", "Status": "状态", "Stopped": "已停止", "Network clients can search note content and obtain local paths without authentication.": "网络客户端无需身份验证即可搜索笔记内容并获取本机文件路径。", "system": "跟随系统", "default": "默认", "light": "浅色", "dark": "深色", "english": "English", "chinese": "简体中文", "visual-studio-code": "VS Code", "Select a note in the sidebar.": "在侧栏选择一篇笔记。", "Open a Markdown library to start reading.": "打开 Markdown 文档库开始阅读。", "No visible notes. Notes use a directory containing index.md.": "没有可见笔记。每篇笔记是包含 index.md 的目录。",
}
