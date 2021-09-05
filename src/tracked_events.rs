#[derive(Clone, Copy)]
pub enum EventType {
    NullEvent,
    CpuRead{address: u16, data: u8},
    CpuWrite{address: u16, data: u8},
}

#[derive(Clone, Copy)]
pub struct TrackedEvent {
    pub scanline: u16,
    pub cycle: u16,
    pub event_type: EventType
}

pub struct EventTracker {
    pub tracked_events_a: Vec<TrackedEvent>,
    pub size_a: usize,
    pub tracked_events_b: Vec<TrackedEvent>,
    pub size_b: usize,
    pub a_active: bool,
}

impl EventTracker {
    pub fn new() -> EventTracker {
        return EventTracker {
            // Way, way more events than we could *possibly* need, just to be safe
            // Manually indexed, and never resized, to avoid allocations at runtime
            tracked_events_a: vec![TrackedEvent{scanline: 0xFFFF, cycle: 0xFFFF, event_type: EventType::NullEvent}; 262*341],
            size_a: 0,
            tracked_events_b: vec![TrackedEvent{scanline: 0xFFFF, cycle: 0xFFFF, event_type: EventType::NullEvent}; 262*341],
            size_b: 0,
            a_active: true,
        }
    }

    pub fn track(&mut self, event: TrackedEvent) {
        match self.a_active {
            true => {
                self.tracked_events_a[self.size_a] = event;
                self.size_a += 1;
            },
            false => {
                self.tracked_events_b[self.size_b] = event;
                self.size_b += 1;  
            }
        }
    }

    pub fn swap_buffers(&mut self) {
        match self.a_active {
            true => {
                self.size_b = 0;
                self.a_active = false;
            },
           false => {
                self.size_a = 0;
                self.a_active = true;
            }
        }
    }

    pub fn events_this_frame(&self) -> &[TrackedEvent] {
        match self.a_active {
            true => &self.tracked_events_a[..self.size_a],
            false => &self.tracked_events_b[..self.size_b],
        }
    }

    pub fn events_last_frame(&self) -> &[TrackedEvent] {
        match self.a_active {
            true => &self.tracked_events_b[..self.size_b],
            false => &self.tracked_events_a[..self.size_a],
        }
    }
}