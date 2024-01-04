// Bridbe between the org.freedesktop.ScreenSaver interface and the Wayland idle
// inhibitor protocol.

mod xdg_screensaver;
mod wayland;

use std::collections::HashMap;

use dbus::channel::MatchingReceiver;
use dbus::message::MatchRule;
use dbus_crossroads::Crossroads;
use dbus_tokio::connection;
use futures::future;
use wayland::InhibitorManager;
use wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1;
use xdg_screensaver::OrgFreedesktopScreenSaver;

#[derive(Debug)]
struct StoredInhibitor {
    inhibitor: ZwpIdleInhibitorV1,
    name: String,
    reason: String,
}

#[derive(Debug)]
struct OrgFreedesktopScreenSaverServer {
    inhibit_manager: InhibitorManager,
    inhibitors_by_cookie: HashMap<u32, StoredInhibitor>,
}

impl OrgFreedesktopScreenSaverServer {
    fn insert_inhibitor(&mut self, inhibitor: StoredInhibitor) -> u32 {
        // atomically find and insert a random cookie that doesn't exist yet
        // TODO: how is this thread safe?
        let cookie = loop {
            let cookie = rand::random();
            if !self.inhibitors_by_cookie.contains_key(&cookie) {
                break cookie;
            }
        };
        self.inhibitors_by_cookie.insert(cookie, inhibitor);
        cookie
    }
}

impl OrgFreedesktopScreenSaver for OrgFreedesktopScreenSaverServer {
    fn inhibit(
        &mut self,
        application_name: String,
        reason_for_inhibit: String,
    ) -> Result<u32, dbus::MethodErr> {
        // TODO: create a surface??
        let inhibitor = self.inhibit_manager.create_inhibitor();
        let cookie = self.insert_inhibitor(StoredInhibitor {
            inhibitor,
            name: application_name,
            reason: reason_for_inhibit,
        });

        return Ok(cookie);
    }

    fn un_inhibit(&mut self, cookie: u32) -> Result<(), dbus::MethodErr> {
        let inhibitor = self.inhibitors_by_cookie.remove(&cookie);
        if let Some(inhibitor) = inhibitor {
            self.inhibit_manager.destroy_inhibitor(inhibitor.inhibitor);
        }

        return Ok(());
    }
}

#[tokio::main(flavor = "multi_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the D-Bus session bus (this is blocking, unfortunately).
    let (resource, c) = connection::new_session_sync()?;

    // The resource is a task that should be spawned onto a tokio compatible
    // reactor ASAP. If the resource ever finishes, you lost connection to D-Bus.
    //
    // To shut down the connection, both call _handle.abort() and drop the connection.
    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    // Let's request a name on the bus, so that clients can find us.
    c.request_name("org.freedesktop.ScreenSaver", false, true, false)
        .await?;

    // Create a new crossroads instance.
    // The instance is configured so that introspection and properties interfaces
    // are added by default on object path additions.
    let mut cr = Crossroads::new();

    // Enable async support for the crossroads instance.
    cr.set_async_support(Some((
        c.clone(),
        Box::new(|x| {
            tokio::spawn(x);
        }),
    )));

    let inhibit_manager = wayland::get_inhibit_manager().await?;

    let iface_token = xdg_screensaver::register_org_freedesktop_screen_saver(&mut cr);
    cr.insert(
        "/org/freedesktop/ScreenSaver",
        &[iface_token],
        OrgFreedesktopScreenSaverServer {inhibit_manager, inhibitors_by_cookie: HashMap::new()},
    );

    // We add the Crossroads instance to the connection so that incoming method calls will be handled.
    c.start_receive(
        MatchRule::new_method_call(),
        Box::new(move |msg, conn| {
            cr.handle_message(msg, conn).unwrap();
            true
        }),
    );

    // Run forever.
    future::pending::<()>().await;
    unreachable!()
}
