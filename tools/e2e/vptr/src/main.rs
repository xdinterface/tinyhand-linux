use std::io::BufRead;
use wayland_client::protocol::{wl_pointer, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

#[derive(Default)]
struct State {
    manager: Option<ZwlrVirtualPointerManagerV1>,
    seat: Option<wl_seat::WlSeat>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(state: &mut Self, registry: &wl_registry::WlRegistry, event: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "zwlr_virtual_pointer_manager_v1" => state.manager = Some(registry.bind(name, version.min(2), qh, ())),
                "wl_seat" => state.seat = Some(registry.bind(name, version.min(5), qh, ())),
                _ => {}
            }
        }
    }
}
delegate_noop!(State: ignore wl_seat::WlSeat);
delegate_noop!(State: ZwlrVirtualPointerManagerV1);
delegate_noop!(State: ZwlrVirtualPointerV1);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (w, h): (u32, u32) = (args[1].parse().unwrap(), args[2].parse().unwrap());
    let conn = Connection::connect_to_env().unwrap();
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    conn.display().get_registry(&qh, ());
    let mut state = State::default();
    queue.roundtrip(&mut state).unwrap();
    let pointer = state.manager.as_ref().expect("no virtual pointer protocol").create_virtual_pointer(state.seat.as_ref(), &qh, ());
    queue.roundtrip(&mut state).unwrap();
    let start = std::time::Instant::now();
    let now = || start.elapsed().as_millis() as u32;
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["abs", x, y] => pointer.motion_absolute(now(), x.parse().unwrap(), y.parse().unwrap(), w, h),
            ["down", b] => pointer.button(now(), b.parse().unwrap(), wl_pointer::ButtonState::Pressed),
            ["up", b] => pointer.button(now(), b.parse().unwrap(), wl_pointer::ButtonState::Released),
            ["sleep", ms] => std::thread::sleep(std::time::Duration::from_millis(ms.parse().unwrap())),
            _ => eprintln!("vptr: ? {line}"),
        }
        pointer.frame();
        queue.roundtrip(&mut state).unwrap();
    }
}
