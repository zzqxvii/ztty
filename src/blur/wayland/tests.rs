use std::{
    os::unix::net::UnixStream,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use wayland_protocols::ext::background_effect::v1::server::{
    ext_background_effect_manager_v1 as manager, ext_background_effect_surface_v1 as effect,
};
use wayland_server::{
    Client, DataInit, Dispatch as Serve, Display, DisplayHandle, GlobalDispatch, New, Resource,
    backend::ClientData,
    protocol::{wl_compositor, wl_region, wl_surface},
};

use super::*;

/// The rects a region holds.
type Rects = Mutex<Vec<[i32; 4]>>;

/// A compositor that writes down what the window asks of it.
struct Compositor {
    heard: Arc<Mutex<Vec<String>>>,
}

impl Compositor {
    fn hear(&self, line: String) {
        self.heard.lock().expect("the notes").push(line);
    }
}

struct Peer;

impl ClientData for Peer {}

impl GlobalDispatch<wl_compositor::WlCompositor, ()> for Compositor {
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<wl_compositor::WlCompositor>,
        _: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        init.init(resource, ());
    }
}

impl Serve<wl_compositor::WlCompositor, ()> for Compositor {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_compositor::WlCompositor,
        request: wl_compositor::Request,
        _: &(),
        _: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                init.init(id, ());
            }
            wl_compositor::Request::CreateRegion { id } => {
                init.init(id, Rects::default());
            }
            _ => {}
        }
    }
}

impl Serve<wl_surface::WlSurface, ()> for Compositor {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_surface::WlSurface,
        _: wl_surface::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
    }
}

impl Serve<wl_region::WlRegion, Rects> for Compositor {
    fn request(
        _: &mut Self,
        _: &Client,
        _: &wl_region::WlRegion,
        request: wl_region::Request,
        rects: &Rects,
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        if let wl_region::Request::Add {
            x,
            y,
            width,
            height,
        } = request
        {
            rects.lock().expect("the rects").push([x, y, width, height]);
        }
    }
}

impl GlobalDispatch<manager::ExtBackgroundEffectManagerV1, ()> for Compositor {
    fn bind(
        _: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<manager::ExtBackgroundEffectManagerV1>,
        _: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        init.init(resource, ())
            .capabilities(manager::Capability::Blur);
    }
}

impl Serve<manager::ExtBackgroundEffectManagerV1, ()> for Compositor {
    fn request(
        compositor: &mut Self,
        _: &Client,
        _: &manager::ExtBackgroundEffectManagerV1,
        request: manager::Request,
        _: &(),
        _: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        if let manager::Request::GetBackgroundEffect { id, surface } = request {
            init.init(id, ());
            compositor.hear(format!("effect on surface {}", surface.id().protocol_id()));
        }
    }
}

impl Serve<effect::ExtBackgroundEffectSurfaceV1, ()> for Compositor {
    fn request(
        compositor: &mut Self,
        _: &Client,
        _: &effect::ExtBackgroundEffectSurfaceV1,
        request: effect::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        if let effect::Request::SetBlurRegion { region } = request {
            let line = match region {
                Some(region) => {
                    let rects = region.data::<Rects>().expect("a region's rects");
                    format!("blur {:?}", rects.lock().expect("the rects"))
                }
                None => "blur nothing".to_string(),
            };
            compositor.hear(line);
        }
    }
}

/// The test's own client, holding the display and a surface where gpui would.
struct Gpui;

impl Dispatch<WlRegistry, GlobalListContents> for Gpui {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(Gpui: WlCompositor);
delegate_noop!(Gpui: ignore WlSurface);

/// Runs a compositor on `socket` until `stop`, offering `ext_background_effect_v1` when `offered`.
fn serve(
    socket: UnixStream,
    offered: bool,
    heard: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut display = Display::<Compositor>::new().expect("a display");
        let mut handle = display.handle();
        handle.create_global::<Compositor, wl_compositor::WlCompositor, ()>(6, ());
        if offered {
            handle.create_global::<Compositor, manager::ExtBackgroundEffectManagerV1, ()>(1, ());
        }
        handle
            .insert_client(socket, Arc::new(Peer))
            .expect("the window connects");
        let mut compositor = Compositor { heard };
        while !stop.load(Ordering::Acquire) {
            display.dispatch_clients(&mut compositor).expect("requests");
            display.flush_clients().expect("events");
            thread::sleep(Duration::from_millis(1));
        }
    })
}

/// What a compositor heard once `ask` used an effect on a fresh surface, and that surface's id.
fn heard(offered: bool, ask: impl FnOnce(Option<Effect>)) -> (Vec<String>, u32) {
    crate::testlog::init();
    let (window_end, compositor_end) = UnixStream::pair().expect("a socket pair");
    let notes = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let server = serve(compositor_end, offered, notes.clone(), stop.clone());
    let connection = Connection::from_socket(window_end).expect("libwayland connects");
    let (globals, mut queue) = registry_queue_init::<Gpui>(&connection).expect("globals");
    let compositor: WlCompositor =
        (globals.bind(&queue.handle(), 1..=6, ())).expect("a compositor");
    let surface = compositor.create_surface(&queue.handle(), ());
    let display = NonNull::new(connection.backend().display_ptr().cast()).expect("a display");
    let pointer = NonNull::new(surface.id().as_ptr().cast()).expect("a surface");
    // SAFETY: the test's live display, which outlives the effect, and its live surface.
    ask(unsafe { Effect::new(display, pointer) }.expect("no Wayland failure"));
    queue
        .roundtrip(&mut Gpui)
        .expect("the compositor heard all");
    stop.store(true, Ordering::Release);
    server.join().expect("the compositor ends");
    let notes = notes.lock().expect("the notes").clone();
    (notes, surface.id().protocol_id())
}

#[test]
fn the_window_s_surface_blurs_while_asked() {
    let (notes, surface) = heard(true, |effect| {
        let mut effect = effect.expect("the compositor offers the protocol");
        effect.blur(Some([10, 10, 300, 200]));
        effect.blur(Some([10, 10, 300, 200]));
        effect.blur(Some([0, 0, 320, 220]));
        effect.blur(None);
    });
    let expected = [
        format!("effect on surface {surface}"),
        "blur [[10, 10, 300, 200]]".into(),
        "blur [[0, 0, 320, 220]]".into(),
        "blur nothing".into(),
    ];
    assert_eq!(notes, expected, "a repeat sends nothing");
    assert!(crate::testlog::logged(
        "blur: ext_background_effect_v1 on the window's surface; the compositor blurs: true, org_kde_kwin_blur too: false"
    ));
}

#[test]
fn without_the_protocol_nothing_is_asked() {
    let (notes, _) = heard(false, |effect| assert!(effect.is_none()));
    assert!(notes.is_empty(), "{notes:?}");
    assert!(crate::testlog::logged(
        "blur: the compositor offers neither ext_background_effect_v1 nor org_kde_kwin_blur; a see-through window shows the desktop clear"
    ));
}
