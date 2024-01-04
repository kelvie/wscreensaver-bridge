// get a wayland client

use futures::future::poll_fn;
use wayland_client::{
    protocol::{
        wl_registry::{self, WlRegistry},
        wl_surface::WlSurface, wl_compositor::WlCompositor,
    },
    Dispatch,
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    __interfaces::ZWP_IDLE_INHIBIT_MANAGER_V1_INTERFACE,
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
    zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};

#[derive(Debug, Default)]
struct DispatcherListener {
    manager: Option<ZwpIdleInhibitManagerV1>,
    dummy_surface: Option<WlSurface>,
}

// TODO: how do I move these into another file?
impl Dispatch<WlCompositor, ()> for DispatcherListener {
    fn event(
        state: &mut Self,
        proxy: &WlCompositor,
        event: <WlCompositor as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitorV1, ()> for DispatcherListener {
    fn event(
        state: &mut Self,
        proxy: &ZwpIdleInhibitorV1,
        event: <ZwpIdleInhibitorV1 as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for DispatcherListener {
    fn event(
        state: &mut Self,
        proxy: &ZwpIdleInhibitManagerV1,
        event: <ZwpIdleInhibitManagerV1 as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlRegistry, ()> for DispatcherListener {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        data: &(),
        conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == ZWP_IDLE_INHIBIT_MANAGER_V1_INTERFACE.name {
                    let manager =
                        registry.bind::<ZwpIdleInhibitManagerV1, _, _>(name, version, qhandle, ());
                    state.manager = Some(manager);
                    // signal to stop waiting
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub(crate) struct InhibitorManager {
    manager: ZwpIdleInhibitManagerV1,
    dummy_surface: WlSurface,
    queue_handle: wayland_client::QueueHandle<DispatcherListener>,
}

impl InhibitorManager {
    pub fn create_inhibitor(&self) -> ZwpIdleInhibitorV1 {
        self.manager
            .create_inhibitor(&self.dummy_surface, &self.queue_handle, ())
    }

    pub fn destroy_inhibitor(&self, inhibitor: ZwpIdleInhibitorV1) {
        inhibitor.destroy();
    }
}

pub async fn get_inhibit_manager() -> Result<InhibitorManager, Box<dyn std::error::Error>> {
    // get wayland display
    let conn =
        wayland_client::Connection::connect_to_env().expect("Failed to connect to Wayland server");
    let mut event_queue = conn.new_event_queue();
    let display = conn.display();
    let qh = event_queue.handle();
    // registry is returned in the dispatch
    let _ = display.get_registry(&qh, ());

    let mut dl = DispatcherListener::default();

    loop {
        poll_fn(|ctx| event_queue.poll_dispatch_pending(ctx, &mut dl)).await?;
        if let (Some(manager), Some(dummy_surface)) = (dl.manager, dl.dummy_surface) {
            return Ok(InhibitorManager {
                manager,
                dummy_surface,
                queue_handle: qh,
            })
        }
    }
}
