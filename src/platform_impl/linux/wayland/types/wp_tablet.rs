use dpi::{LogicalPosition, PhysicalPosition};
use sctk::globals::GlobalData;
use sctk::reexports::client::globals::{BindError, GlobalList};
use sctk::reexports::client::protocol::{wl_seat::WlSeat, wl_surface::WlSurface};
use sctk::reexports::client::{
    delegate_dispatch, event_created_child, Connection, Dispatch, Proxy, QueueHandle,
};
use sctk::reexports::protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2::ZwpTabletManagerV2,
    zwp_tablet_pad_group_v2::{ZwpTabletPadGroupV2, EVT_RING_OPCODE, EVT_STRIP_OPCODE},
    zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2,
    zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2,
    zwp_tablet_pad_v2::{Event as PadEvent, ZwpTabletPadV2, EVT_GROUP_OPCODE},
    zwp_tablet_seat_v2::{
        Event as SeatEvent, ZwpTabletSeatV2, EVT_PAD_ADDED_OPCODE, EVT_TABLET_ADDED_OPCODE,
        EVT_TOOL_ADDED_OPCODE,
    },
    zwp_tablet_tool_v2::{ButtonState, Event as ToolEvent, Type as WlToolType, ZwpTabletToolV2},
    zwp_tablet_v2::{Event as TabletEvent, ZwpTabletV2},
};
use sctk::shell::xdg::window::Window;
use wayland_client::WEnum;

use crate::event::{
    ButtonSource, DeviceId, ElementState, Force, PointerKind, PointerSource, ToolButton, ToolState, ToolTilt, ToolType, WindowEvent
};
use crate::platform_impl::wayland;
use crate::platform_impl::wayland::state::WinitState;

#[derive(Debug)]
pub struct TabletManager {
    manager: ZwpTabletManagerV2,
}

#[derive(Debug, Default)]
pub struct ToolData {
    device_id: i64,
    surface: Option<WlSurface>,
    rotation: Option<f64>,
    tilt: Option<(f64, f64)>,
    position: Option<(f64, f64)>,
    pressure: Option<u32>,
    typ: Option<WlToolType>,
    moved: bool,
    entered: bool,
    left: bool,
    button_events: Vec<(ToolButton, ElementState)>,
}

impl TabletManager {
    pub fn new(
        globals: &GlobalList,
        queue_handle: &QueueHandle<WinitState>,
    ) -> Result<Self, BindError> {
        let manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(Self { manager })
    }

    pub fn tablet_seat(
        &self,
        seat: &WlSeat,
        queue_handle: &QueueHandle<WinitState>,
    ) -> ZwpTabletSeatV2 {
        self.manager.get_tablet_seat(seat, queue_handle, ())
    }
}

impl Dispatch<ZwpTabletManagerV2, GlobalData, WinitState> for TabletManager {
    fn event(
        _: &mut WinitState,
        _: &ZwpTabletManagerV2,
        _: <ZwpTabletManagerV2 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        // No events.
    }
}

impl Dispatch<ZwpTabletSeatV2, (), WinitState> for TabletManager {
    fn event(
        state: &mut WinitState,
        _: &ZwpTabletSeatV2,
        event: SeatEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Tablet seat event: {event:?}");
        match event {
            SeatEvent::ToolAdded { id } => {
                let mut data: ToolData = Default::default();
                data.device_id = state.tools.len() as i64;
                state.tools.insert(id.id(), data);
            },
            _ => (),
        }
    }

    event_created_child!(WinitState, ZwpTabletSeatV2, [
        EVT_TABLET_ADDED_OPCODE => (ZwpTabletV2, ()),
        EVT_TOOL_ADDED_OPCODE => (ZwpTabletToolV2, ()),
        EVT_PAD_ADDED_OPCODE => (ZwpTabletPadV2, ()),
    ]);
}

impl Dispatch<ZwpTabletV2, (), WinitState> for TabletManager {
    fn event(
        _: &mut WinitState,
        tablet: &ZwpTabletV2,
        event: TabletEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Tablet event: {event:?}");
        match event {
            TabletEvent::Removed => tablet.destroy(),
            _ => (),
        }
    }
}

impl Dispatch<ZwpTabletToolV2, (), WinitState> for TabletManager {
    fn event(
        state: &mut WinitState,
        tool: &ZwpTabletToolV2,
        event: ToolEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Tool event: {event:?} {}", tool.id());
        let id = tool.id();
        let data = state.tools.get_mut(&id).unwrap();
        match event {
            ToolEvent::Type { tool_type: WEnum::Value(typ) } => {
                data.typ = Some(typ); // TODO how to report this?
            },
            ToolEvent::ProximityIn { surface, .. } => {
                data.surface = Some(surface);
                data.entered = true;
            },
            ToolEvent::ProximityOut => {
                data.left = true;
            },
            ToolEvent::Pressure { pressure } => {
                data.pressure = Some(pressure);
                data.moved = true;
            },
            ToolEvent::Rotation { degrees } => {
                data.rotation = Some(degrees);
                data.moved = true;
            },
            ToolEvent::Down { .. } => {
                data.button_events.push((ToolButton::Contact, ElementState::Pressed));
            },
            ToolEvent::Up => {
                data.button_events.push((ToolButton::Contact, ElementState::Released));
            },
            ToolEvent::Tilt { tilt_x, tilt_y } => {
                data.tilt = Some((tilt_x, tilt_y));
                data.moved = true;
            },
            ToolEvent::Motion { x, y } => {
                data.position = Some((x, y));
                data.moved = true;
            },
            ToolEvent::Button { button, state: WEnum::Value(btn_state), .. } => {
                let button = match button {
                    0x14b /* BTN_STYLUS */ => ToolButton::Button1,
                    0x14c /* BTN_STYLUS2 */ => ToolButton::Button2,
                    0x149 /* BTN_STYLUS3 */ => ToolButton::Button3,
                    _ => ToolButton::Other(button as u16),
                };
                let state = match btn_state {
                    ButtonState::Pressed => ElementState::Pressed,
                    _ => ElementState::Released,
                };
                data.button_events.push((button, state));
            },
            ToolEvent::Frame { .. } => {
                let window_id = wayland::make_wid(data.surface.as_ref().unwrap());
                let scale_factor = match state.windows.borrow().get(&window_id) {
                    Some(window) => window.lock().unwrap().scale_factor(),
                    None => return,
                };
                let position: PhysicalPosition<f64> = match data.position {
                    Some((x, y)) => LogicalPosition::new(x, y).to_physical(scale_factor),
                    _ => return,
                };
                let device_id = Some(DeviceId::from_raw(data.device_id));
                // TODO should we handle the others, too?
                let typ = match data.typ {
                    Some(WlToolType::Eraser) => ToolType::Eraser,
                    _ => ToolType::Pen
                };
                if data.entered {
                    data.entered = false;
                    state.events_sink.push_window_event(
                        WindowEvent::PointerEntered {
                            device_id,
                            position,
                            primary: true,
                            kind: PointerKind::Tool(typ),
                        },
                        window_id,
                    );
                }
                if data.moved {
                    data.moved = false;
                    let source = PointerSource::Tool(ToolState {
                        force: data
                            .pressure
                            .map(|p| Force::Normalized((p as f64) / 65535.))
                            .unwrap_or(Force::Normalized(1.)),
                        tangential_force: None,
                        twist: data.rotation,
                        tilt: data.tilt.map(|(x, y)| ToolTilt { x, y }),
                        angle: None,
                        typ
                    });
                    state.events_sink.push_window_event(
                        WindowEvent::PointerMoved { device_id, position, primary: true, source },
                        window_id,
                    );
                }
                if data.left {
                    data.left = false;
                    state.events_sink.push_window_event(
                        WindowEvent::PointerLeft {
                            device_id,
                            position: Some(position),
                            primary: true,
                            kind: PointerKind::Tool(typ),
                        },
                        window_id,
                    );
                }
                data.button_events.clear();
            },
            ToolEvent::Removed => {
                state.tools.remove(&id).unwrap();
                tool.destroy();
            },
            _ => (),
        }
    }
}

impl Dispatch<ZwpTabletPadV2, (), WinitState> for TabletManager {
    fn event(
        _: &mut WinitState,
        pad: &ZwpTabletPadV2,
        event: PadEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Pad event: {event:?}");
        match event {
            PadEvent::Removed => pad.destroy(),
            _ => (),
        }
    }

    event_created_child!(WinitState, ZwpTabletPadV2, [
        EVT_GROUP_OPCODE => (ZwpTabletPadGroupV2, ())
    ]);
}

impl Dispatch<ZwpTabletPadGroupV2, (), WinitState> for TabletManager {
    fn event(
        _: &mut WinitState,
        _: &ZwpTabletPadGroupV2,
        event: <ZwpTabletPadGroupV2 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Group event: {event:?}");
    }

    event_created_child!(WinitState, ZwpTabletPadGroupV2, [
        EVT_STRIP_OPCODE => (ZwpTabletPadStripV2, ()),
        EVT_RING_OPCODE => (ZwpTabletPadRingV2, ()),
    ]);
}

impl Dispatch<ZwpTabletPadRingV2, (), WinitState> for TabletManager {
    fn event(
        _: &mut WinitState,
        _: &ZwpTabletPadRingV2,
        event: <ZwpTabletPadRingV2 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Ring event: {event:?}");
    }
}

impl Dispatch<ZwpTabletPadStripV2, (), WinitState> for TabletManager {
    fn event(
        _: &mut WinitState,
        _: &ZwpTabletPadStripV2,
        event: <ZwpTabletPadStripV2 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Strip event: {event:?}");
    }
}

delegate_dispatch!(WinitState: [ZwpTabletManagerV2: GlobalData] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletSeatV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletToolV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletPadV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletPadGroupV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletPadRingV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletPadStripV2: ()] => TabletManager);
