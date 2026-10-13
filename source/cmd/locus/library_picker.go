package main

import (
	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/shells/sidebar"
)

// The replacement page leaves the workspace's sidebar presentation untouched.
func (a *application) libraryPicker(c *ui.Context) {
	a.flushNotices(c)
	ui.Column(c).Fill().Padding(sidebar.HeaderHeight(c), 32, 32, 32).
		AlignItems(ui.Center).Justify(ui.Center).Gap(16).Children(func() {
		components.Text(c, "Locus").Bold()
		components.Text(c, a.text("Choose a library")).Bold()
		components.Text(c, a.text("Open a Markdown library to start reading.")).Wrap().TextColor(c.Theme().TextMuted)
		components.Button(c, a.text("Open library"), components.ButtonPrimary).Disabled(a.choosing).OnClick(a.chooseLibrary)
		components.Button(c, a.text("Settings"), components.ButtonGhost).OnClick(a.toggleSettings)
	})
	a.commandDialog(c)
	a.settingsDialog(c)
}
