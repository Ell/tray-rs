use tray::platform::Platform;
use tray::{Tray, TrayItem, TrayMenu};

fn main() {
    let mut tray = Tray::new();

    let mut menu = TrayMenu::new();

    menu.add_item().label("Enabled".to_string());

    menu.add_item().disable().label("Disabled".to_string());

    menu.add_divider();

    menu.add_item()
        .label("Toggle Checkbox".to_string())
        .on_click(|item: &mut TrayItem| {
            item.toggle_checked();
        });

    menu.add_divider();

    let submenu = menu
        .add_item()
        .label("Test Submenu".to_string())
        .create_submenu();

    submenu
        .add_item()
        .label("Submenu Entry 1".to_string())
        .on_click(|_| println!("clicked!"));

    submenu.add_item().label("Submenu Entry 2".to_string());

    menu.add_divider();

    menu.add_item().label("Quit".to_string()).on_click(|_| {});

    tray.add_menu(menu).unwrap();

    let platform = Platform::new();

    tray.platform(platform);

    if tray.run().is_err() {
        panic!("platform error")
    }
}
