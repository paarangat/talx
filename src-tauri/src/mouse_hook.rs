use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    XButton1,
    XButton2,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonEvent {
    Pressed,
    Released,
}

static EVENT_SENDER: Mutex<Option<Sender<MouseButtonEvent>>> = Mutex::new(None);
static ACTIVE_BUTTON: Mutex<Option<MouseButton>> = Mutex::new(None);

pub fn parse_button(s: &str) -> Result<MouseButton, String> {
    match s {
        "xbutton1" => Ok(MouseButton::XButton1),
        "xbutton2" => Ok(MouseButton::XButton2),
        "middle" => Ok(MouseButton::Middle),
        _ => Err(format!("Unknown mouse button: {s}")),
    }
}

pub fn start(button: MouseButton) -> Result<(MouseHook, Receiver<MouseButtonEvent>), String> {
    let (tx, rx) = mpsc::channel();
    *EVENT_SENDER.lock().unwrap() = Some(tx);
    *ACTIVE_BUTTON.lock().unwrap() = Some(button);

    let (stop_tx, stop_rx) = mpsc::channel();

    let handle = std::thread::Builder::new()
        .name("mouse-hook".into())
        .spawn(move || {
            run_hook(stop_rx);
            *EVENT_SENDER.lock().unwrap() = None;
        })
        .map_err(|e| format!("Failed to spawn mouse hook thread: {e}"))?;

    Ok((
        MouseHook {
            handle: Some(handle),
            stop_tx,
        },
        rx,
    ))
}

unsafe extern "system" fn hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code >= 0 {
        let msg = w_param as u32;
        let info = l_param as *const MSLLHOOKSTRUCT;

        let button = ACTIVE_BUTTON.lock().unwrap();
        let sender = EVENT_SENDER.lock().unwrap();

        if let (Some(btn), Some(ref sender)) = (*button, sender.clone()) {
            let is_match = match msg {
                WM_XBUTTONDOWN | WM_XBUTTONUP => {
                    let xbutton = ((*info).mouse_data >> 16) as u16;
                    matches!(
                        (btn, xbutton),
                        (MouseButton::XButton1, 1) | (MouseButton::XButton2, 2)
                    )
                }
                WM_MBUTTONDOWN | WM_MBUTTONUP => btn == MouseButton::Middle,
                _ => false,
            };

            if is_match {
                let event = match msg {
                    WM_XBUTTONDOWN | WM_MBUTTONDOWN => MouseButtonEvent::Pressed,
                    _ => MouseButtonEvent::Released,
                };
                let _ = sender.send(event);
            }
        }
    }

    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

fn run_hook(stop_rx: Receiver<()>) {
    unsafe {
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), std::ptr::null_mut(), 0);
        if hook.is_null() {
            return;
        }

        let mut msg: MSG = std::mem::zeroed();
        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }

            if PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }

        UnhookWindowsHookEx(hook);
    }
}

pub struct MouseHook {
    handle: Option<std::thread::JoinHandle<()>>,
    stop_tx: Sender<()>,
}

impl Drop for MouseHook {
    fn drop(&mut self) {
        self.stop();
    }
}

impl MouseHook {
    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = self.stop_tx.send(());
            let _ = handle.join();
        }
    }
}

type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type BOOL = i32;
type DWORD = u32;
type HHOOK = *mut std::ffi::c_void;
type HMODULE = *mut std::ffi::c_void;
type HWND = *mut std::ffi::c_void;
type HOOKPROC = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

const WH_MOUSE_LL: i32 = 14;
const PM_REMOVE: u32 = 1;
const WM_XBUTTONDOWN: u32 = 0x020B;
const WM_XBUTTONUP: u32 = 0x020C;
const WM_MBUTTONDOWN: u32 = 0x0207;
const WM_MBUTTONUP: u32 = 0x0208;
const WM_QUIT: u32 = 0x0012;

#[repr(C)]
struct MSLLHOOKSTRUCT {
    pt: POINT,
    mouse_data: u32,
    flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
    time: u32,
    pt: POINT,
}

extern "system" {
    fn SetWindowsHookExW(
        id_hook: i32,
        lpfn: Option<HOOKPROC>,
        hmod: HMODULE,
        dw_thread_id: DWORD,
    ) -> HHOOK;
    fn CallNextHookEx(hhk: HHOOK, n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn UnhookWindowsHookEx(hhk: HHOOK) -> BOOL;
    fn PeekMessageW(
        lp_msg: *mut MSG,
        h_wnd: HWND,
        w_msg_filter_min: u32,
        w_msg_filter_max: u32,
        w_remove_msg: u32,
    ) -> BOOL;
    fn TranslateMessage(lp_msg: *const MSG) -> BOOL;
    fn DispatchMessageW(lp_msg: *const MSG) -> LRESULT;
}
