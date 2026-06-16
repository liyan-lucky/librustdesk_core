use hbb_common::message_proto::CursorData;
use hbb_common::ResultType;

pub const PA_SAMPLE_RATE: u32 = 48000;
pub const SERVICE_INTERVAL: u64 = 300;

#[derive(Default)]
pub struct WakeLock;

impl WakeLock {
    pub fn new(_display: bool, _idle: bool, _sleep: bool) -> Self {
        Self
    }
}

pub fn installing_service() -> bool {
    false
}

pub fn is_xfce() -> bool {
    false
}

pub fn breakdown_callback() {}

pub fn get_wakelock(_display: bool) -> WakeLock {
    WakeLock
}

pub fn get_active_username() -> String {
    crate::username()
}

pub fn is_prelogin() -> bool {
    false
}

pub fn is_installed() -> bool {
    false
}

pub fn change_resolution(_name: &str, _width: usize, _height: usize) -> ResultType<()> {
    Ok(())
}

pub fn get_cursor_pos() -> ResultType<(i32, i32)> {
    Ok((0, 0))
}

pub fn set_cursor_pos(_x: i32, _y: i32) -> ResultType<()> {
    Ok(())
}

pub fn get_cursor() -> ResultType<Option<usize>> {
    Ok(None)
}

pub fn get_cursor_data(_cursor: usize) -> ResultType<CursorData> {
    Ok(CursorData::new())
}

pub fn clip_cursor(_x: i32, _y: i32, _w: i32, _h: i32) -> ResultType<()> {
    Ok(())
}

pub fn get_focused_display() -> ResultType<String> {
    Ok(String::new())
}

pub fn start_os_service() {}

pub mod linux {
    pub const DISPLAY_SERVER_X11: &str = "x11";
    pub const DISPLAY_SERVER_WAYLAND: &str = "wayland";
    pub const CMD_SH: &str = "sh";

    pub fn is_x11() -> bool {
        false
    }

    pub fn is_login_screen_wayland() -> bool {
        false
    }

    pub fn is_login_wayland() -> bool {
        false
    }

    pub fn current_is_wayland() -> bool {
        false
    }

    pub fn get_display_server() -> &'static str {
        DISPLAY_SERVER_X11
    }

    pub fn get_default_pa_source() -> Option<(String, String)> {
        None
    }

    pub fn get_pa_source_name(device: &str) -> String {
        device.to_owned()
    }

    pub fn get_pa_monitor() -> String {
        String::new()
    }

    pub fn get_pa_sources() -> Vec<String> {
        Vec::new()
    }

    pub fn get_active_userid() -> u32 {
        0
    }

    pub fn get_active_userid_fresh() -> u32 {
        0
    }

    pub fn is_selinux_enforcing() -> bool {
        false
    }

    pub fn has_gnome_shortcuts_inhibitor_permission() -> bool {
        false
    }

    pub fn clear_gnome_shortcuts_inhibitor_permission() -> hbb_common::ResultType<()> {
        Ok(())
    }
}

pub mod linux_desktop_manager {
    pub fn start_xdesktop() {}

    pub fn get_username() -> String {
        String::new()
    }

    pub fn is_headless() -> bool {
        false
    }

    pub fn try_start_desktop(_username: &str, _password: &str) -> hbb_common::ResultType<()> {
        Ok(())
    }
}

pub mod gtk_sudo {
    pub fn exec() {}

    pub fn run(_cmd: Vec<&str>) -> hbb_common::ResultType<()> {
        Ok(())
    }
}
