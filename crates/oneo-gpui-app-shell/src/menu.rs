use gpui::{App, KeyBinding, Menu, MenuItem, SharedString, actions};

actions!(
    app_shell,
    [
        About,
        CloseWindow,
        OpenSettings,
        Quit,
        ToggleFpsCounter,
        ToggleFullScreen,
        ToggleLeftPanel,
        ToggleRightPanel
    ]
);

pub struct AppShellMenuLabels {
    pub about: SharedString,
    pub close_window: SharedString,
    pub developer: SharedString,
    pub edit: SharedString,
    pub file: SharedString,
    pub quit: SharedString,
    pub settings: SharedString,
    pub toggle_left_panel: SharedString,
    pub toggle_right_panel: SharedString,
    pub toggle_fps_counter: SharedString,
    pub toggle_full_screen: SharedString,
    pub view: SharedString,
    pub window: SharedString,
}

pub struct AppShellMenus {
    pub app_name: SharedString,
    pub labels: AppShellMenuLabels,
    pub application_items: Vec<MenuItem>,
    pub edit_items: Vec<MenuItem>,
    pub file_items: Vec<MenuItem>,
    pub view_items_before_panels: Vec<MenuItem>,
    pub view_items_after_panels: Vec<MenuItem>,
}

impl AppShellMenus {
    pub fn new(app_name: impl Into<SharedString>, labels: AppShellMenuLabels) -> Self {
        let app_name = app_name.into();
        Self {
            labels,
            app_name,
            application_items: Vec::new(),
            edit_items: Vec::new(),
            file_items: Vec::new(),
            view_items_before_panels: Vec::new(),
            view_items_after_panels: Vec::new(),
        }
    }

    pub fn application_items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.application_items.extend(items);
        self
    }

    pub fn file_items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.file_items.extend(items);
        self
    }

    pub fn edit_items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.edit_items.extend(items);
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
    ]);
    #[cfg(target_os = "macos")]
    cx.bind_keys([KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None)]);
    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([KeyBinding::new("f11", ToggleFullScreen, None)]);
    #[cfg(debug_assertions)]
    cx.bind_keys([KeyBinding::new(
        "secondary-alt-shift-f",
        ToggleFpsCounter,
        None,
    )]);
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
    ]);
    if !menus.view_items_after_panels.is_empty() {
        view_items.push(MenuItem::separator());
        view_items.extend(menus.view_items_after_panels);
    }
    view_items.extend([
        MenuItem::separator(),
        MenuItem::action(labels.toggle_full_screen, ToggleFullScreen),
    ]);

    let mut native_menus = vec![Menu::new(menus.app_name).items(application_items)];
    if !menus.file_items.is_empty() {
        native_menus.push(Menu::new(labels.file).items(menus.file_items));
    }
    if !menus.edit_items.is_empty() {
        native_menus.push(Menu::new(labels.edit).items(menus.edit_items));
    }
    native_menus.push(Menu::new(labels.view).items(view_items));
    #[cfg(debug_assertions)]
    native_menus.push(Menu::new(labels.developer).items([MenuItem::action(
        labels.toggle_fps_counter,
        ToggleFpsCounter,
    )]));
    native_menus
        .push(Menu::new(labels.window).items([MenuItem::action(labels.close_window, CloseWindow)]));
    cx.set_menus(native_menus);
}
