use crate::platform::TrayPlatform;
use crate::platform::TrayPlatformError;
use crate::TrayItem;
use crate::TrayMenu;
use std::cell::RefCell;
use std::io::Error;
use std::ptr::null_mut;
use std::sync::Arc;
use winapi::shared::windef::HBITMAP;
use winapi::shared::windef::POINT;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::Shell_NotifyIconW;
use winapi::um::winuser::CreatePopupMenu;
use winapi::um::winuser::CreateWindowExW;
use winapi::um::winuser::DefWindowProcW;
use winapi::um::winuser::DestroyMenu;
use winapi::um::winuser::DestroyWindow;
use winapi::um::winuser::DispatchMessageW;
use winapi::um::winuser::GetCursorPos;
use winapi::um::winuser::GetMenuItemInfoW;
use winapi::um::winuser::GetMessageW;
use winapi::um::winuser::InsertMenuItemW;
use winapi::um::winuser::LoadCursorW;
use winapi::um::winuser::LoadIconW;
use winapi::um::winuser::PostQuitMessage;
use winapi::um::winuser::RegisterClassW;
use winapi::um::winuser::SetForegroundWindow;
use winapi::um::winuser::SetMenuInfo;
use winapi::um::winuser::TrackPopupMenu;
use winapi::um::winuser::TranslateMessage;
use winapi::um::winuser::MFS_CHECKED;
use winapi::um::winuser::MFS_DISABLED;
use winapi::um::winuser::MFT_STRING;
use winapi::um::winuser::MIIM_DATA;
use winapi::um::winuser::MIIM_FTYPE;
use winapi::um::winuser::MIIM_ID;
use winapi::um::winuser::MIIM_STATE;
use winapi::um::winuser::MIIM_STRING;
use winapi::um::winuser::MIIM_SUBMENU;
use winapi::um::winuser::MSG;
use winapi::um::winuser::TPM_BOTTOMALIGN;
use winapi::um::winuser::TPM_LEFTALIGN;
use winapi::um::winuser::{WM_LBUTTONUP, WM_QUIT, WM_RBUTTONUP};

use winapi::{
    ctypes::{c_ulong, c_ushort},
    shared::{
        basetsd::ULONG_PTR,
        guiddef::GUID,
        minwindef::{DWORD, HINSTANCE, LPARAM, LRESULT, UINT, WPARAM},
        ntdef::LPCWSTR,
        windef::{HBRUSH, HICON, HMENU, HWND},
    },
    um::{
        shellapi::{NIF_MESSAGE, NIM_ADD, NOTIFYICONDATAW},
        winuser::{
            self, CW_USEDEFAULT, MENUINFO, MENUITEMINFOW, MIM_APPLYTOSUBMENUS, MIM_STYLE,
            MNS_NOTIFYBYPOS, WM_CLOSE, WM_DESTROY, WM_MENUCOMMAND, WM_USER, WNDCLASSW,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

type Result<T> = std::result::Result<T, TrayPlatformError>;

const WM_TRAY_CALLBACK: u32 = WM_USER + 1;

thread_local! {
    static APPLICATIONMENU: RefCell<Menu> = RefCell::new(Menu::new());
}

impl TrayPlatformError {
    pub fn from_win32_error(msg: &str) -> Self {
        let last_error = Error::last_os_error();

        Self {
            details: format!("{}: {}", &msg, last_error),
        }
    }
}

unsafe fn create_notify_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as DWORD,
        hWnd: hwnd,
        uID: 0x1 as UINT,
        uFlags: 0 as UINT,
        uCallbackMessage: 0 as UINT,
        hIcon: 0 as HICON,
        szTip: [0 as u16; 128],
        dwState: 0 as DWORD,
        dwStateMask: 0 as DWORD,
        szInfo: [0 as u16; 256],
        u: Default::default(),
        szInfoTitle: [0 as u16; 64],
        dwInfoFlags: 0 as UINT,
        guidItem: GUID {
            Data1: 0 as c_ulong,
            Data2: 0 as c_ushort,
            Data3: 0 as c_ushort,
            Data4: [0; 8],
        },
        hBalloonIcon: 0 as HICON,
    }
}

unsafe fn create_hmenu() -> Result<HMENU> {
    let hmenu = CreatePopupMenu();

    let menu_info = MENUINFO {
        cbSize: std::mem::size_of::<MENUINFO>() as DWORD,
        fMask: MIM_APPLYTOSUBMENUS | MIM_STYLE,
        dwStyle: MNS_NOTIFYBYPOS,
        cyMax: 0 as UINT,
        hbrBack: 0 as HBRUSH,
        dwContextHelpID: 0 as DWORD,
        dwMenuData: 0 as ULONG_PTR,
    };

    if SetMenuInfo(hmenu, &menu_info as *const MENUINFO) == 0 {
        return Err(TrayPlatformError::from_win32_error("Error setting up menu"));
    }

    Ok(hmenu)
}

unsafe fn create_hwnd(class_name: &str, hinstance: HINSTANCE) -> Result<HWND> {
    let name = to_wstring(class_name);

    let wnd = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(window_proc),
        hInstance: hinstance,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hCursor: LoadCursorW(0 as HINSTANCE, winuser::IDI_APPLICATION),
        hIcon: LoadIconW(0 as HINSTANCE, winuser::IDI_APPLICATION),
        hbrBackground: 16 as HBRUSH,
        lpszMenuName: 0 as LPCWSTR,
        lpszClassName: name.as_ptr(),
    };

    if RegisterClassW(&wnd) == 0 {
        return Err(TrayPlatformError::from_win32_error(
            "Error creating window class",
        ));
    }

    let hwnd = CreateWindowExW(
        0,
        name.as_ptr(),
        to_wstring("tray_rs_tray").as_ptr(),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        0,
        CW_USEDEFAULT,
        0,
        0 as HWND,
        0 as HMENU,
        0 as HINSTANCE,
        null_mut(),
    );

    if hwnd.is_null() {
        return Err(TrayPlatformError::from_win32_error("Error creating window"));
    }

    Ok(hwnd)
}

fn to_wstring(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAY_CALLBACK => {
            let param = lparam as UINT;

            match param {
                WM_LBUTTONUP => {}
                WM_RBUTTONUP => {
                    let mut point = POINT { x: 0, y: 0 };

                    if GetCursorPos(&mut point as *mut POINT) == 0 {
                        return 1;
                    }

                    SetForegroundWindow(hwnd);

                    APPLICATIONMENU.with(|appmenu| {
                        let hmenu = appmenu.borrow_mut().build().unwrap().hmenu.unwrap();

                        TrackPopupMenu(
                            hmenu,
                            0,
                            point.x,
                            point.y,
                            (TPM_BOTTOMALIGN | TPM_LEFTALIGN) as i32,
                            hwnd,
                            null_mut(),
                        );
                    })
                }
                _ => {}
            }
        }

        WM_MENUCOMMAND => APPLICATIONMENU.with(|appmenu| {
            let menuitem: Box<MENUITEMINFOW> = Box::new(MENUITEMINFOW {
                fMask: MIIM_ID | MIIM_DATA,
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as DWORD,
                ..Default::default()
            });

            let menuitem_ptr = Box::into_raw(menuitem);

            let lpmii = menuitem_ptr as *mut MENUITEMINFOW;

            let hmenu = appmenu.borrow_mut().build().unwrap().hmenu.unwrap();

            let u_item = wparam as *mut u32;

            dbg!(u_item);

            if GetMenuItemInfoW(hmenu, *u_item, 0, lpmii) == 1 {
                println!("get succeed");

                let tray_item = menuitem_ptr.as_mut().unwrap().dwItemData as *mut TrayItem;

                if let Some(callback) = &(*tray_item).callback {
                    callback(tray_item.as_mut().unwrap())
                }
            } else {
                let error = TrayPlatformError::from_win32_error("Could not get menu item info");

                println!("{:?}", error);

                panic!(error);
            };
        }),

        WM_CLOSE => {
            DestroyWindow(hwnd);

            return 0;
        }

        WM_DESTROY => {
            PostQuitMessage(0);
        }

        _ => {}
    };

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[derive(Default)]
struct Menu {
    pub hmenu: Option<HMENU>,
    pub traymenu: Option<Arc<RefCell<TrayMenu>>>,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn traymenu(&mut self, traymenu: Arc<RefCell<TrayMenu>>) -> &mut Self {
        self.traymenu = Some(traymenu);

        self
    }

    pub unsafe fn build(&mut self) -> Result<&mut Self> {
        if let Some(hmenu) = self.hmenu {
            DestroyMenu(hmenu);
        }

        let traymenu = self.traymenu.as_ref().unwrap();

        let hmenu = self.build_hmenu(&traymenu.borrow(), 0).unwrap();

        self.hmenu = Some(hmenu);

        Ok(self)
    }

    unsafe fn build_hmenu(&self, traymenu: &TrayMenu, idx: u32) -> Result<HMENU> {
        let hmenu = create_hmenu().unwrap();

        for (i, item) in &mut traymenu.items.iter().rev().enumerate() {
            let id = (i as u32 + idx + 1000) as u32;

            let menu_item_data = Box::into_raw(Box::new(traymenu)) as ULONG_PTR;

            let mut hmenu_item = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as UINT,
                fMask: MIIM_FTYPE | MIIM_STRING | MIIM_ID | MIIM_STATE,
                fType: MFT_STRING,
                fState: 0 as UINT,
                wID: id,
                hSubMenu: 0 as HMENU,
                hbmpChecked: 0 as HBITMAP,
                hbmpUnchecked: 0 as HBITMAP,
                dwItemData: menu_item_data,
                dwTypeData: null_mut(),
                cch: 0 as u32,
                hbmpItem: 0 as HBITMAP,
            };

            if let Some(l) = &item.label {
                let mut label = to_wstring(l);

                hmenu_item.dwTypeData = label.as_mut_ptr();
                hmenu_item.cch = (label.len() * 2) as u32;
            }

            if let Some(submenu) = &item.submenu {
                hmenu_item.fMask |= MIIM_SUBMENU;
                hmenu_item.hSubMenu = self.build_hmenu(submenu, id).unwrap();
            }

            if item.disabled {
                hmenu_item.fState |= MFS_DISABLED;
            }

            if item.checked {
                hmenu_item.fState |= MFS_CHECKED;
            }

            if InsertMenuItemW(hmenu, 0, 1, &hmenu_item as *const MENUITEMINFOW) == 0 {
                return Err(TrayPlatformError::from_win32_error(
                    "Error inserting menu item",
                ));
            }
        }

        Ok(hmenu)
    }
}

#[derive(Default)]
struct App {}

impl App {
    pub unsafe fn init() -> Result<Self> {
        let hinstance = GetModuleHandleW(null_mut());

        let hwnd = create_hwnd("tray_rs_window", hinstance).unwrap();

        let mut icon_data = create_notify_icon_data(hwnd);
        icon_data.uID = 0x1;
        icon_data.uFlags = NIF_MESSAGE;
        icon_data.uCallbackMessage = WM_TRAY_CALLBACK;

        if Shell_NotifyIconW(NIM_ADD, &mut icon_data as *mut NOTIFYICONDATAW) == 0 {
            return Err(TrayPlatformError::from_win32_error(
                "Error adding menu icon",
            ));
        }

        Ok(Self {})
    }

    pub unsafe fn update(&self) -> Result<()> {
        let mut msg = MSG {
            ..Default::default()
        };

        GetMessageW(&mut msg, 0 as HWND, 0, 0);

        if msg.message == WM_QUIT {
            return Err(TrayPlatformError::new("quit event recieved"));
        }

        TranslateMessage(&msg);
        DispatchMessageW(&msg);

        Ok(())
    }
}

#[derive(Default)]
pub struct Platform {
    app: Option<App>,
}

impl Platform {
    pub fn new() -> Self {
        Platform {
            ..Default::default()
        }
    }
}

impl TrayPlatform for Platform {
    fn update(&self) -> Result<()> {
        Ok(())
    }

    fn quit(&self) -> Result<()> {
        Ok(())
    }

    fn run(&self) -> Result<()> {
        let app = self.app.as_ref();

        unsafe {
            loop {
                app.unwrap().update().unwrap();
            }
        }
    }

    fn init(&mut self, traymenu: Arc<RefCell<TrayMenu>>) -> Result<()> {
        unsafe {
            let app = App::init().unwrap();

            self.app = Some(app);

            APPLICATIONMENU.with(|appmenu| {
                appmenu.borrow_mut().traymenu(traymenu);
            });
        }

        Ok(())
    }
}
