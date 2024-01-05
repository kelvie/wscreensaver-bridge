// Bridbe between the org.freedesktop.ScreenSaver interface and the Wayland idle
// inhibitor protocol.

mod xdg_screensaver;
mod wayland;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
        // find an insert a new cookie. we're locked so this should be gucci
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

impl OrgFreedesktopScreenSaver for Arc<Mutex<OrgFreedesktopScreenSaverServer>> {
    fn inhibit(
        &mut self,
        application_name: String,
        reason_for_inhibit: String,
    ) -> Result<(u32,), dbus::MethodErr> {
        log::info!("Inhibiting screensaver for {:?} because {:?}", application_name, reason_for_inhibit);
        let inhibitor = self.lock().unwrap().inhibit_manager.create_inhibitor();
        let cookie = self.lock().unwrap().insert_inhibitor(StoredInhibitor {
            inhibitor,
            name: application_name,
            reason: reason_for_inhibit,
        });

        return Ok((cookie,));
    }

    fn un_inhibit(&mut self, cookie: u32) -> Result<(), dbus::MethodErr> {
        log::info!("Uninhibiting {:?}", cookie);
        let inhibitor = self.lock().unwrap().inhibitors_by_cookie.remove(&cookie);
        log::info!("Inhibitor found? {:?}", inhibitor);
        if let Some(inhibitor) = inhibitor {
            self.lock().unwrap().inhibit_manager.destroy_inhibitor(inhibitor.inhibitor);
        }

        return Ok(());
    }
}

#[tokio::main(flavor = "multi_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // configure logger to print thread id
    let mut log_builder = pretty_env_logger::formatted_builder();
    log_builder.format(|buf, record| {
        use std::io::Write;
        writeln!(
            buf,
            "[{:?}][{}] {}",
            std::thread::current().id(),
            record.level(),
            record.args()
        )
    });

    log_builder.filter_level(log::LevelFilter::Info);

    log_builder.init();

    log::info!("Starting screensaver bridge");

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

    let mut cr = Crossroads::new();

    // Enable async support for the crossroads instance.
    cr.set_async_support(Some((
        c.clone(),
        Box::new(|x| {
            // spawn and put a log statement inside
            tokio::spawn(async move {
                x.await;
            });
        }),
    )));

    log::info!("Waiting for wayland compositor");
    let inhibit_manager = wayland::get_inhibit_manager().await?;

    let iface_token = xdg_screensaver::register_org_freedesktop_screen_saver(&mut cr);
    cr.insert(
        "/org/freedesktop/ScreenSaver",
        &[iface_token],
        Arc::new(Mutex::new(OrgFreedesktopScreenSaverServer {
            inhibit_manager,
            inhibitors_by_cookie: HashMap::new(),
        })),
    );

    // TODO: list the inhibitors that are active

    log::log!(log::Level::Info, "Starting ScreenSaver to Wayland bridge");
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
