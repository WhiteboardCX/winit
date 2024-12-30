use sctk::globals::GlobalData;
use sctk::reexports::client::globals::{BindError, GlobalList};
use sctk::reexports::client::protocol::wl_seat::WlSeat;
use sctk::reexports::client::{
    delegate_dispatch, event_created_child, Connection, Dispatch, Proxy, QueueHandle,
};
use sctk::reexports::protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2::ZwpTabletManagerV2,
    zwp_tablet_pad_group_v2::{ZwpTabletPadGroupV2, EVT_RING_OPCODE, EVT_STRIP_OPCODE},
    zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2,
    zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2,
    zwp_tablet_pad_v2::{ZwpTabletPadV2, EVT_GROUP_OPCODE},
    zwp_tablet_seat_v2::{
        ZwpTabletSeatV2, EVT_PAD_ADDED_OPCODE, EVT_TABLET_ADDED_OPCODE, EVT_TOOL_ADDED_OPCODE,
    },
    zwp_tablet_tool_v2::ZwpTabletToolV2,
    zwp_tablet_v2::ZwpTabletV2,
};

use crate::platform_impl::wayland::state::WinitState;

#[derive(Debug)]
pub struct TabletManager {
    manager: ZwpTabletManagerV2,
}

pub struct TabletSeatData {
    seat: WlSeat,
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
        let data = TabletSeatData { seat: seat.clone() };
        self.manager.get_tablet_seat(seat, queue_handle, data)
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

impl Dispatch<ZwpTabletSeatV2, TabletSeatData, WinitState> for TabletManager {
    fn event(
        state: &mut WinitState,
        _: &ZwpTabletSeatV2,
        event: <ZwpTabletSeatV2 as Proxy>::Event,
        data: &TabletSeatData,
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("TabletSeatEvent {event:?}");
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
        _: &ZwpTabletV2,
        event: <ZwpTabletV2 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Tablet event: {event:?}");
    }
}

impl Dispatch<ZwpTabletToolV2, (), WinitState> for TabletManager { fn event(
        _: &mut WinitState,
        tool: &ZwpTabletToolV2,
        event: <ZwpTabletToolV2 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Tool event: {event:?} {}", tool.id());
    }
}

impl Dispatch<ZwpTabletPadV2, (), WinitState> for TabletManager {
    fn event(
        _: &mut WinitState,
        _: &ZwpTabletPadV2,
        event: <ZwpTabletPadV2 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WinitState>,
    ) {
        println!("Pad event: {event:?}");
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

impl Dispatch<ZwpTabletPadRingV2, (), WinitState> for TabletManager { fn event(
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

impl Dispatch<ZwpTabletPadStripV2, (), WinitState> for TabletManager { fn event(
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
delegate_dispatch!(WinitState: [ZwpTabletSeatV2: TabletSeatData] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletToolV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletPadV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletPadGroupV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletPadRingV2: ()] => TabletManager);
delegate_dispatch!(WinitState: [ZwpTabletPadStripV2: ()] => TabletManager);
