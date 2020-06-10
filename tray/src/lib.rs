pub mod platform;

use crate::platform::TrayPlatform;
use std::cell::RefCell;
use std::cell::RefMut;
use std::rc::Rc;

type Result<T> = std::result::Result<T, TrayError>;

#[derive(Debug, Clone)]
pub struct TrayError {
    details: String,
}

impl TrayError {
    fn new(msg: &str) -> TrayError {
        TrayError {
            details: msg.to_string(),
        }
    }
}

impl std::fmt::Display for TrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Tray Error: {}", self.details)
    }
}

impl std::error::Error for TrayError {
    fn description(&self) -> &str {
        &self.details
    }
}

#[derive(Default)]
pub struct Tray {
    pub platform: Option<Box<dyn TrayPlatform>>,
    pub menu: Rc<RefCell<TrayMenu>>,
}

impl Tray {
    pub fn new() -> Self {
        Tray {
            platform: None,
            menu: Rc::new(RefCell::new(TrayMenu::new())),
        }
    }

    pub fn platform<T: TrayPlatform + 'static>(&mut self, platform: T) -> &mut Self {
        self.platform = Some(Box::new(platform));

        self
    }

    pub fn add_menu(&mut self, menu: TrayMenu) -> RefMut<TrayMenu> {
        let menu = Rc::new(RefCell::new(menu));

        self.menu = menu;

        self.menu.borrow_mut()
    }

    pub fn run(&mut self) -> Result<()> {
        let platform = self.platform.as_mut().unwrap();

        platform.init(self.menu.clone()).unwrap();

        match platform.run() {
            Err(e) => Err(TrayError::new(&e.details)),
            _ => Ok(()),
        }
    }
}

pub struct TrayIcon {
    buffer: Vec<u8>,
}

impl TrayIcon {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self { buffer }
    }
}

#[derive(Default)]
pub struct TrayMenu {
    items: Vec<TrayItem>,
}

impl TrayMenu {
    pub fn new() -> Self {
        TrayMenu { items: vec![] }
    }

    pub fn add_divider(&mut self) {
        let mut item = TrayItem::new();
        item.divider(true);

        self.items.push(item);
    }

    pub fn add_item(&mut self) -> &mut TrayItem {
        self.items.push(TrayItem::new());

        self.items.last_mut().unwrap()
    }
}

pub type TrayItemCallback = dyn Fn(&mut TrayItem) -> ();

#[derive(Default)]
pub struct TrayItem {
    label: Option<String>,
    divider: Option<bool>,
    disabled: bool,
    checked: Option<bool>,
    submenu: Option<TrayMenu>,
    callback: Option<Box<TrayItemCallback>>,
}

impl TrayItem {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn disable(&mut self) -> &mut Self {
        self.disabled = true;

        self
    }

    pub fn enable(&mut self) -> &mut Self {
        self.disabled = false;

        self
    }

    pub fn create_submenu(&mut self) -> &mut TrayMenu {
        let menu = TrayMenu::new();

        self.submenu = Some(menu);

        self.submenu.as_mut().unwrap()
    }

    pub fn divider(&mut self, divider: bool) -> &mut Self {
        self.divider = Some(divider);

        self
    }

    pub fn checkbox(&mut self, checked: bool) -> &mut Self {
        match self.checked {
            Some(_) => (),
            _ => self.checked = Some(checked),
        }

        self
    }

    pub fn toggle(&mut self) -> &mut Self {
        self.disabled = !self.disabled;

        self
    }

    pub fn toggle_checked(&mut self) -> &mut Self {
        self.checked = Some(!self.checked.unwrap());

        self
    }

    pub fn label(&mut self, label: String) -> &mut Self {
        self.label = Some(label);

        self
    }

    pub fn on_click<T: Fn(&mut Self) -> () + 'static>(&mut self, callback: T) -> &mut Self {
        self.callback = Some(Box::new(callback));

        self
    }
}
