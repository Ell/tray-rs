#[macro_use]
extern crate educe;

pub mod platform;

use crate::platform::TrayPlatform;
use std::cell::RefCell;
use std::fmt::Debug;
use std::sync::Arc;

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

#[derive(Default, Debug)]
pub struct Tray {
    pub platform: Option<Box<dyn TrayPlatform>>,
    pub menu: Arc<RefCell<TrayMenu>>,
}

impl Tray {
    pub fn new() -> Self {
        Tray {
            platform: None,
            menu: Arc::new(RefCell::new(TrayMenu::new())),
        }
    }

    pub fn platform<T: TrayPlatform + 'static>(&mut self, platform: T) -> &mut Self {
        self.platform = Some(Box::new(platform));

        self
    }

    pub fn add_menu(&mut self, menu: TrayMenu) -> Result<()> {
        self.menu = Arc::new(RefCell::new(menu));

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        let platform = self.platform.as_mut().unwrap();

        platform.init(self.menu.clone()).unwrap();

        println!("{:?}", self.menu);

        match platform.run() {
            Err(e) => Err(TrayError::new(&e.details)),
            _ => Ok(()),
        }
    }

    pub fn quit(self) {}
}

pub struct TrayIcon {
    buffer: Vec<u8>,
}

impl TrayIcon {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self { buffer }
    }
}

#[derive(Default, Debug)]
pub struct TrayMenu {
    pub items: Vec<TrayItem>,
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

impl Drop for TrayMenu {
    fn drop(&mut self) {
        println!("traymenu killed {:?}", self);
    }
}

pub type TrayItemCallback = dyn Fn(&mut TrayItem) -> ();

#[derive(Educe)]
#[educe(Debug, Default)]
pub struct TrayItem {
    label: Option<String>,
    divider: bool,
    disabled: bool,
    checked: bool,
    submenu: Option<TrayMenu>,

    #[educe(Debug(ignore))]
    callback: Option<Box<TrayItemCallback>>,
}

impl TrayItem {
    pub fn new() -> Self {
        Self {
            disabled: false,
            checked: false,
            divider: false,
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
        self.divider = divider;

        self
    }

    pub fn toggle(&mut self) -> &mut Self {
        self.disabled = !self.disabled;

        self
    }

    pub fn toggle_checked(&mut self) -> &mut Self {
        self.checked = !self.checked;

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
