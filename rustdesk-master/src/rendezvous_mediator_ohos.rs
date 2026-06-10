use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) static NEEDS_DEPLOY: AtomicBool = AtomicBool::new(false);
static LAN_LISTENER_STARTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct RendezvousMediator;

impl RendezvousMediator {
    pub fn restart() {
        hbb_common::log::info!("OHOS rendezvous mediator restart requested");
        start_lan_listener_once();
    }

    pub async fn start_all() {
        start_lan_listener_once();
        std::future::pending::<()>().await;
    }
}

fn start_lan_listener_once() {
    if LAN_LISTENER_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    hbb_common::log::info!("OHOS rendezvous mediator starting LAN listener");
    std::thread::spawn(move || {
        crate::harmony_bridge::core::queue_event("lan-listener", "starting", "");
        if let Err(err) = crate::lan::start_listening() {
            LAN_LISTENER_STARTED.store(false, Ordering::SeqCst);
            hbb_common::log::error!("LAN listener failed: {}", err);
            crate::harmony_bridge::core::queue_event("lan-listener", &format!("failed: {}", err), "");
        }
    });
}

pub(crate) fn reset_needs_deploy_notification() {
    NEEDS_DEPLOY.store(false, Ordering::SeqCst);
}
