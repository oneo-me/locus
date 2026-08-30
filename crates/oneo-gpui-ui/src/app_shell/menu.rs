use gpui::{App, KeyBinding, Menu, MenuItem, SharedString, actions};

actions!(
    app_shell,
    [
        About,
        CloseWindow,
        OpenSettings,
        Quit,
        ToggleFpsCounter,
        ToggleLeftPanel,
        ToggleRightPanel
    ]
);

pub struct AppShellMenuLabels {
    pub about: SharedString,
    pub close_window: SharedString,
    pub edit: SharedString,
    pub quit: SharedString,
    pub settings: SharedString,
    pub toggle_left_panel: SharedString,
    pub toggle_right_panel: SharedString,
    pub toggle_fps_counter: SharedString,
    pub view: SharedString,
    pub window: SharedString,
}

impl Default for AppShellMenuLabels {
    fn default() -> Self {
        Self {
            about: "About".into(),
            close_window: "Close Window".into(),
            edit: "Edit".into(),
            quit: "Quit".into(),
            settings: "Settings…".into(),
            toggle_left_panel: "Toggle Left Panel".into(),
            toggle_right_panel: "Toggle Right Panel".into(),
            toggle_fps_counter: "Show FPS".into(),
            view: "View".into(),
            window: "Window".into(),
        }
    }
}

impl AppShellMenuLabels {
    fn for_app(app_name: &str) -> Self {
        Self {
            about: format!("About {app_name}").into(),
            quit: format!("Quit {app_name}").into(),
            ..Self::default()
        }
    }
}

pub struct AppShellMenus {
    pub app_name: SharedString,
    pub labels: AppShellMenuLabels,
    pub application_items: Vec<MenuItem>,
    pub view_items_before_panels: Vec<MenuItem>,
    pub view_items_after_panels: Vec<MenuItem>,
}

impl AppShellMenus {
    pub fn new(app_name: impl Into<SharedString>) -> Self {
        let app_name = app_name.into();
        Self {
            labels: AppShellMenuLabels::for_app(&app_name),
            app_name,
            application_items: Vec::new(),
            view_items_before_panels: Vec::new(),
            view_items_after_panels: Vec::new(),
        }
    }

    pub fn labels(mut self, labels: AppShellMenuLabels) -> Self {
        self.labels = labels;
        self
    }

    pub fn application_items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.application_items.extend(items);
        self
    }

    pub fn view_items_before_panels(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.view_items_before_panels.extend(items);
        self
    }

    pub fn view_items_after_panels(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.view_items_after_panels.extend(items);
        self
    }
}

pub fn bind_app_shell_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-q", Quit, None),
        KeyBinding::new("secondary-w", CloseWindow, None),
        KeyBinding::new("secondary-,", OpenSettings, None),
        KeyBinding::new("secondary-b", ToggleLeftPanel, None),
        KeyBinding::new("secondary-shift-b", ToggleRightPanel, None),
        KeyBinding::new("secondary-alt-shift-f", ToggleFpsCounter, None),
    ]);
}

pub fn set_app_shell_menus(cx: &mut App, menus: AppShellMenus) {
    let labels = menus.labels;
    let mut application_items = Vec::new();
    #[cfg(target_os = "macos")]
    application_items.extend([
        MenuItem::action(labels.about.clone(), About),
        MenuItem::separator(),
    ]);
    application_items.extend(menus.application_items);
    if !application_items.is_empty() {
        application_items.push(MenuItem::separator());
    }
    application_items.extend([
        MenuItem::action(labels.settings, OpenSettings),
        MenuItem::separator(),
        MenuItem::action(labels.quit, Quit),
    ]);

    let mut view_items = menus.view_items_before_panels;
    if !view_items.is_empty() {
        view_items.push(MenuItem::separator());
    }
    view_items.extend([
        MenuItem::action(labels.toggle_left_panel, ToggleLeftPanel),
        MenuItem::action(labels.toggle_right_panel, ToggleRightPanel),
        MenuItem::separator(),
        MenuItem::action(labels.toggle_fps_counter, ToggleFpsCounter),
    ]);
    if !menus.view_items_after_panels.is_empty() {
        view_items.push(MenuItem::separator());
        view_items.extend(menus.view_items_after_panels);
    }

    cx.set_menus([
        Menu::new(menus.app_name).items(application_items),
        Menu::new(labels.edit).disabled(true),
        Menu::new(labels.view).items(view_items),
        Menu::new(labels.window).items([MenuItem::action(labels.close_window, CloseWindow)]),
    ]);
}
