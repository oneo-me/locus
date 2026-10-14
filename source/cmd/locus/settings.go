package main

import (
	"os"

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

func (a *application) languageSetting(c *ui.Context) {
	control := components.Select(c, &a.cfg.Language)
	control.Trigger.Label(a.text("Language")).Width(c.Theme().Space(40)).Shrink(0).Children(func() {
		components.Text(c, a.languageLabel(a.cfg.Language)).SingleLine()
	})
	control.Popup(func(ui.Element) {
		for _, option := range languageOptions {
			control.Item(option).Label(a.languageLabel(option)).Children(func() {
				components.Text(c, a.languageLabel(option)).SingleLine()
			})
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
	width := min(c.Theme().Space(195), w-2*inset)
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
			item("language", "Language", func(c *ui.Context) { a.languageSetting(c) }),
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
