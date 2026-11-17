//! Reads the active tab of the front-most Chrome window through Windows UI Automation.
//! No extension, no Chrome flags: the same accessibility tree screen readers use.

use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, PartialEq, Debug)]
pub struct Tab {
    pub title: String,
    pub url: String,
    /// Browser process; its whole tree is captured in live mode.
    pub pid: u32,
}

/// Poll in the background and send the tab whenever it changes.
pub fn watch(tx: UnboundedSender<Option<Tab>>) {
    std::thread::spawn(move || {
        #[cfg(windows)]
        let ua = uiautomation::UIAutomation::new().ok();
        let mut last = None;
        loop {
            #[cfg(windows)]
            let tab = ua.as_ref().and_then(active_tab);
            #[cfg(not(windows))]
            let tab: Option<Tab> = None; // ponytail: macOS via AppleScript when it ships there
            if tab != last {
                if tx.send(tab.clone()).is_err() {
                    break;
                }
                last = tab;
            }
            std::thread::sleep(Duration::from_millis(1200));
        }
    });
}

#[cfg(windows)]
const BROWSERS: [&str; 4] = [" - Google Chrome", " - Brave", " - Microsoft\u{200b} Edge", " - Microsoft Edge"];

#[cfg(windows)]
fn active_tab(ua: &uiautomation::UIAutomation) -> Option<Tab> {
    use uiautomation::patterns::UIValuePattern;
    use uiautomation::types::ControlType;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetClassNameW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible};
    use windows::core::BOOL;

    // EnumWindows walks top-level windows front to back, so the first Chrome hit is the one the user last used.
    unsafe extern "system" fn first_chrome(h: HWND, out: LPARAM) -> BOOL {
        unsafe {
            let out = &mut *(out.0 as *mut Option<(isize, String)>);
            if !IsWindowVisible(h).as_bool() {
                return true.into();
            }
            let mut buf = [0u16; 512];
            let n = GetClassNameW(h, &mut buf) as usize;
            if String::from_utf16_lossy(&buf[..n]) != "Chrome_WidgetWin_1" {
                return true.into();
            }
            let n = GetWindowTextW(h, &mut buf) as usize;
            let text = String::from_utf16_lossy(&buf[..n]);
            // Chromium browsers share Chrome's window class and accessibility tree.
            match BROWSERS.iter().find_map(|s| text.strip_suffix(s)) {
                Some(title) => {
                    *out = Some((h.0 as isize, title.to_string()));
                    false.into()
                }
                None => true.into(),
            }
        }
    }

    let mut found: Option<(isize, String)> = None;
    unsafe {
        let _ = EnumWindows(Some(first_chrome), LPARAM(&mut found as *mut _ as isize));
    }
    let (hwnd, title) = found?;
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut pid)) };
    let window = ua.element_from_handle(hwnd.into()).ok()?;
    // The omnibox is the first Edit control; matching by type, not name, keeps it language-independent.
    let omnibox = ua.create_matcher().from(window).control_type(ControlType::Edit).depth(30).timeout(300).find_first().ok()?;
    let mut url = omnibox.get_pattern::<UIValuePattern>().ok()?.get_value().ok()?;
    if url.trim().is_empty() || url.contains(' ') {
        return None; // new tab page, or the user is typing a search
    }
    if !url.contains("://") {
        url = format!("https://{url}");
    }
    Some(Tab { title: clean_title(&title), url, pid })
}

/// "(3) Some Video - YouTube" -> "Some Video"
fn clean_title(t: &str) -> String {
    let t = match t.strip_prefix('(').and_then(|r| r.split_once(") ")) {
        Some((n, rest)) if n.chars().all(|c| c.is_ascii_digit()) => rest,
        _ => t,
    };
    t.strip_suffix(" - YouTube").unwrap_or(t).to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn titles() {
        assert_eq!(super::clean_title("(3) Rust in 100 Seconds - YouTube"), "Rust in 100 Seconds");
        assert_eq!(super::clean_title("(draft) notes"), "(draft) notes");
    }
}
