// get a wayland client

use wayland_client::{
    protocol::{
        __interfaces::WL_COMPOSITOR_INTERFACE,
        wl_compositor::WlCompositor,
        wl_registry::{self, WlRegistry},
        wl_surface::WlSurface,
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
impl Dispatch<WlSurface, ()> for DispatcherListener {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        _event: <WlSurface as wayland_client::Proxy>::Event,
        _data: &(),
        _connn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlCompositor, ()> for DispatcherListener {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitorV1, ()> for DispatcherListener {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitorV1,
        _event: <ZwpIdleInhibitorV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for DispatcherListener {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitManagerV1,
        _event: <ZwpIdleInhibitManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<WlRegistry, ()> for DispatcherListener {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == WL_COMPOSITOR_INTERFACE.name {
                    log::debug!("Found compositor");
                    let compositor =
                        registry.bind::<WlCompositor, _, _>(name, version, qhandle, ());
                    let surface = compositor.create_surface(&qhandle, ());
                    state.dummy_surface = Some(surface);
                }
                if interface == ZWP_IDLE_INHIBIT_MANAGER_V1_INTERFACE.name {
                    log::debug!("Found inhibit manager");
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
        event_queue.blocking_dispatch(&mut dl).unwrap();
        if dl.manager.is_some() && dl.dummy_surface.is_some() {
            return Ok(InhibitorManager {
                manager: dl.manager.take().unwrap(),
                dummy_surface: dl.dummy_surface.take().unwrap(),
                queue_handle: qh,
            });
        }
    }
}
