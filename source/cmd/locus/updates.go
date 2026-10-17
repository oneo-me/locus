package main

import (
	"context"
	"fmt"
	"time"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
	"github.com/egoist/mygo/ui/markdown"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/icons/lucide"
)

const (
	// updateInterval is the time between background checks.
	updateInterval = 24 * time.Hour
	// updateFirstDelay lets launch work finish before the first check.
	updateFirstDelay = 10 * time.Second
	// updateRetry is how often a running app asks whether a check is due.
	updateRetry = time.Hour
	releasesURL = "https://github.com/oneo-me/locus/releases"
)

// updatePhase is where the update flow stands.
type updatePhase int

const (
	updateIdle updatePhase = iota
	updateChecking
	updateCurrent
	updateAvailable
	updateDownloading
	updateReady
	updateFailed
)

// availableUpdate is a newer signed release. install downloads it, checks
// its signature, and replaces the app; the new version runs after a relaunch.
type availableUpdate struct {
	Version, Notes string
	install        func(context.Context, func(downloaded, total int64)) error
}

// updateFlow is the About page's update state. It belongs to the UI thread;
// checks and downloads run in goroutines and post their outcomes back.
type updateFlow struct {
	enabled  bool
	check    func(context.Context) (*availableUpdate, error)
	phase    updatePhase
	found    *availableUpdate
	progress float64
	err      string
	notified string // the version a toast already announced
	notes    markdown.Options
	cancel   context.CancelFunc
	timer    *time.Timer
	// post returns work to the UI thread; nil uses the window. Tests replace it.
	post func(func())
}

// postUpdate returns an outcome of a check or download to the UI thread.
func (a *application) postUpdate(fn func()) {
	if a.updates.post != nil {
		a.updates.post(fn)
	} else {
		a.post(fn)
	}
}

// checkRelease asks MyGo's updater for a newer release than the running one.
func checkRelease(ctx context.Context) (*availableUpdate, error) {
	up, err := mygo.Updater.Check(ctx)
	if err != nil || up == nil {
		return nil, err
	}
	return &availableUpdate{Version: up.Version, Notes: up.Notes, install: up.Install}, nil
}

// startUpdates enables the flow for builds that can replace themselves and
// schedules the first background check. Development builds and package-managed
// installs stay disabled.
func (a *application) startUpdates() {
	a.updates.enabled = mygo.Updater.Enabled()
	a.updates.check = checkRelease
	if a.updates.enabled {
		a.updates.timer = time.AfterFunc(updateFirstDelay, func() { a.postUpdate(a.backgroundUpdateCheck) })
	}
}

// stopUpdates cancels a running check or download and the next check.
func (a *application) stopUpdates() {
	if a.updates.timer != nil {
		a.updates.timer.Stop()
	}
	if a.updates.cancel != nil {
		a.updates.cancel()
	}
}

// backgroundUpdateCheck checks when automatic checks are on and the last
// successful check is a day old, then asks again in an hour.
func (a *application) backgroundUpdateCheck() {
	switch a.updates.phase {
	case updateIdle, updateCurrent, updateFailed:
		if a.cfg.UpdateChecks && time.Since(a.state.UpdateChecked) >= updateInterval {
			a.checkUpdates(false)
		}
	}
	a.updates.timer = time.AfterFunc(updateRetry, func() { a.postUpdate(a.backgroundUpdateCheck) })
}

// checkForUpdates opens About and checks as the user asked.
func (a *application) checkForUpdates() {
	a.openAbout()
	a.checkUpdates(true)
}

// openAbout shows the About page in Settings.
func (a *application) openAbout() {
	a.settingState.Category = "about"
	a.settingState.Query = ""
	a.settingsOpen, a.terminalOpen, a.commandsOpen = true, false, false
	a.invalidate()
}

// checkUpdates asks for a newer release unless a check, download, or pending
// relaunch is under way. A background find is announced, or installed when
// automatic downloads are on; a manual check reports on the About page.
func (a *application) checkUpdates(manual bool) {
	u := &a.updates
	if !u.enabled || u.check == nil {
		return
	}
	switch u.phase {
	case updateChecking, updateDownloading, updateReady:
		return
	}
	ctx, cancel := context.WithCancel(context.Background())
	u.phase, u.err, u.cancel = updateChecking, "", cancel
	a.invalidate()
	check := u.check
	go func() {
		found, err := check(ctx)
		a.postUpdate(func() {
			cancel()
			if err != nil {
				u.phase, u.err = updateFailed, err.Error()
				a.invalidate()
				return
			}
			a.state.UpdateChecked = time.Now()
			if found == nil {
				u.phase, u.found = updateCurrent, nil
				a.invalidate()
				return
			}
			u.phase, u.found = updateAvailable, found
			u.notes = markdown.Options{OnLink: a.openLink}
			a.invalidate()
			switch {
			case !manual && a.cfg.UpdateDownloads:
				a.installUpdate()
			case !manual && u.notified != found.Version:
				u.notified = found.Version
				a.actionNotice(fmt.Sprintf("Locus %s", found.Version), a.text("A new version is available."), a.text("Details"), a.openAbout)
			}
		})
	}()
}

// installUpdate downloads and installs the found release, then offers a
// relaunch. A failure leaves the running app as it was.
func (a *application) installUpdate() {
	u := &a.updates
	if u.phase != updateAvailable || u.found == nil {
		return
	}
	ctx, cancel := context.WithCancel(context.Background())
	u.phase, u.progress, u.err, u.cancel = updateDownloading, 0, "", cancel
	a.invalidate()
	found := u.found
	go func() {
		err := found.install(ctx, func(downloaded, total int64) {
			if total > 0 {
				progress := float64(downloaded) / float64(total)
				a.postUpdate(func() { u.progress = progress; a.invalidate() })
			}
		})
		a.postUpdate(func() {
			cancel()
			if err != nil {
				u.phase, u.err = updateAvailable, err.Error()
				a.errorNotice(a.text("Update failed") + "\n" + err.Error())
				return
			}
			u.phase = updateReady
			a.invalidate()
			if !a.settingsOpen || a.settingState.Category != "about" {
				a.actionNotice(fmt.Sprintf("Locus %s", found.Version), a.text("Restart Locus to finish updating."), a.text("Restart"), mygo.App.Relaunch)
			}
		})
	}()
}

// updatePending reports a release the user has not installed or relaunched
// into yet, which the Settings button highlights.
func (a *application) updatePending() bool {
	return a.updates.phase == updateAvailable || a.updates.phase == updateReady
}

// appVersion is the running version, empty for builds without one.
func appVersion() string {
	if mygo.IsDev() {
		return ""
	}
	return mygo.App.Version()
}

// updateStatus describes the flow in the About page.
func (a *application) updateStatus() string {
	u := &a.updates
	switch {
	case !u.enabled:
		return a.text("This build does not update itself.")
	case u.phase == updateChecking:
		return a.text("Checking for updates…")
	case u.phase == updateAvailable && u.err != "":
		return a.text("Update failed") + " · " + u.err
	case u.phase == updateAvailable:
		return a.text("New version") + " " + u.found.Version
	case u.phase == updateDownloading:
		return a.text("Downloading update…") + fmt.Sprintf(" %d%%", int(u.progress*100))
	case u.phase == updateReady:
		return a.text("Restart Locus to finish updating.")
	case u.phase == updateFailed:
		return a.text("Update failed") + " · " + u.err
	}
	checked := a.text("Not checked yet")
	if !a.state.UpdateChecked.IsZero() {
		checked = a.text("Last checked") + " · " + a.state.UpdateChecked.Local().Format("2006-01-02 15:04")
	}
	if u.phase == updateCurrent {
		return a.text("Locus is up to date.") + " " + checked
	}
	return checked
}

// updateAction builds the control that moves the flow forward.
func (a *application) updateAction(c *ui.Context) {
	u := &a.updates
	switch u.phase {
	case updateChecking:
		components.Button(c, a.text("Checking…"), components.ButtonSecondary).Shrink(0).Disabled(true)
	case updateAvailable:
		components.Button(c, a.text("Download and install"), components.ButtonPrimary).Shrink(0).OnClick(a.installUpdate)
	case updateDownloading:
		components.Progress(c, u.progress).Width(c.Theme().Space(40)).Shrink(0)
	case updateReady:
		components.Button(c, a.text("Restart to update"), components.ButtonPrimary).Shrink(0).OnClick(mygo.App.Relaunch)
	default:
		components.Button(c, a.text("Check for updates"), components.ButtonSecondary).Shrink(0).Disabled(!u.enabled).OnClick(func() { a.checkUpdates(true) })
	}
}

// aboutCategory is the About page: the running version, update status and
// release notes, and the automatic update preferences.
func (a *application) aboutCategory() components.SettingsCategory {
	version := appVersion()
	if version == "" {
		version = a.text("Development build")
	}
	items := []components.SettingsItem{
		{ID: "status", Title: a.text("Software update"), Description: a.updateStatus(), Build: a.updateAction},
	}
	if u := &a.updates; u.found != nil && u.found.Notes != "" && u.phase >= updateAvailable && u.phase <= updateReady {
		items = append(items, components.SettingsItem{ID: "notes", Title: a.text("Release notes"), Description: u.found.Notes, Content: func(c *ui.Context) {
			ui.Column(c).FillWidth().Gap(c.Theme().Space(2)).Children(func() {
				components.Text(c, a.text("Release notes")+" · "+u.found.Version).Wrap()
				markdown.View(c, u.found.Notes, &u.notes).FillWidth()
			})
		}})
	}
	items = append(items,
		components.SettingsItem{ID: "automatic-checks", Title: a.text("Check for updates automatically"), Description: a.text("Locus checks once a day and tells you when a new version is available."), Build: func(c *ui.Context) {
			if components.Switch(c, &a.cfg.UpdateChecks, a.text("Check for updates automatically"), components.SwitchOptions{HideLabel: true}).Changed() {
				a.saveConfig()
			}
		}},
		components.SettingsItem{ID: "automatic-downloads", Title: a.text("Download and install updates automatically"), Description: a.text("Updates found in the background install without asking and run at the next launch."), Build: func(c *ui.Context) {
			if components.Switch(c, &a.cfg.UpdateDownloads, a.text("Download and install updates automatically"), components.SwitchOptions{HideLabel: true}).Changed() {
				a.saveConfig()
			}
		}},
	)
	return components.SettingsCategory{ID: "about", Section: a.text("Application"), Icon: lucide.Info(), Title: a.text("About"), Groups: []components.SettingsGroup{
		{ID: "app", Items: []components.SettingsItem{{ID: "version", Title: "Locus", Description: version, Keywords: []string{a.text("Version")}, Build: func(c *ui.Context) {
			components.Button(c, a.text("Release history"), components.ButtonSecondary).Shrink(0).OnClick(func() { a.openLink(releasesURL) })
		}}}},
		{ID: "updates", Title: a.text("Updates"), Items: items},
	}}
}
