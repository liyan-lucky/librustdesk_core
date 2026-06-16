use hbb_common::message_proto::{ControlKey, KeyEvent, KeyboardMode};

pub fn set_relative_mouse_mode_state(_active: bool) {}

pub fn update_grab_get_key_name(_keyboard_mode: &str) {}

pub fn release_remote_keys(_keyboard_mode: &str) {}

pub mod client {
    use crate::common::GrabState;
    use crate::ui_session_interface::{InvokeUiSession, Session};
    use hbb_common::message_proto::{ControlKey, KeyEvent, KeyboardMode};
    use rdev::Event;

    pub fn change_grab_status(_state: GrabState, _keyboard_mode: &str, _session_id: u128) {}

    pub fn process_event_with_session<T: InvokeUiSession>(
        _keyboard_mode: &str,
        _event: &Event,
        _lock_modes: Option<i32>,
        _session: &Session<T>,
    ) {
    }

    pub fn get_modifiers_state(
        alt: bool,
        ctrl: bool,
        shift: bool,
        command: bool,
    ) -> (bool, bool, bool, bool) {
        (alt, ctrl, shift, command)
    }

    pub fn legacy_modifiers(
        key_event: &mut KeyEvent,
        alt: bool,
        ctrl: bool,
        shift: bool,
        command: bool,
    ) {
        if alt {
            key_event.modifiers.push(ControlKey::Alt.into());
        }
        if ctrl {
            key_event.modifiers.push(ControlKey::Control.into());
        }
        if shift {
            key_event.modifiers.push(ControlKey::Shift.into());
        }
        if command {
            key_event.modifiers.push(ControlKey::Meta.into());
        }
    }

    pub fn event_lock_screen() -> KeyEvent {
        let mut key_event = KeyEvent::new();
        key_event.set_control_key(ControlKey::LockScreen);
        key_event.down = true;
        key_event.mode = KeyboardMode::Legacy.into();
        key_event
    }

    pub fn event_ctrl_alt_del() -> KeyEvent {
        let mut key_event = KeyEvent::new();
        key_event.set_control_key(ControlKey::CtrlAltDel);
        key_event.down = true;
        key_event.mode = KeyboardMode::Legacy.into();
        key_event
    }
}

pub mod input_source {
    pub fn init_input_source() {}

    pub fn change_input_source(_session_id: u128, _input_source: String) {}

    pub fn get_cur_session_input_source() -> String {
        String::new()
    }

    pub fn get_supported_input_source() -> Vec<(String, String)> {
        Vec::new()
    }
}
