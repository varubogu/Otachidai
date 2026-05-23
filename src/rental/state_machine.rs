use dashmap::DashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker},
};

#[derive(Debug)]
pub enum RentalState {
    AwaitingPurpose {
        session_id: i32,
        host_user_id: u64,
        timeout_task: JoinHandle<()>,
    },
    Active {
        session_id: i32,
        host_user_id: u64,
    },
    PendingHandoff {
        session_id: i32,
        timeout_task: JoinHandle<()>,
    },
}

#[derive(Debug)]
pub struct RentalStateEntry {
    pub state: RentalState,
    pub room_id: i32,
}

/// Key: (guild_id raw value, voice_channel_id raw value)
/// We key on voice channel because VC join is the primary trigger.
pub type RentalStateMap = Arc<DashMap<(u64, u64), RentalStateEntry>>;

pub fn new_state_map() -> RentalStateMap {
    Arc::new(DashMap::new())
}

impl RentalStateEntry {
    pub fn session_id(&self) -> i32 {
        match &self.state {
            RentalState::AwaitingPurpose { session_id, .. } => *session_id,
            RentalState::Active { session_id, .. } => *session_id,
            RentalState::PendingHandoff { session_id, .. } => *session_id,
        }
    }

    pub fn abort_timeout(&self) {
        match &self.state {
            RentalState::AwaitingPurpose { timeout_task, .. } => timeout_task.abort(),
            RentalState::PendingHandoff { timeout_task, .. } => timeout_task.abort(),
            RentalState::Active { .. } => {}
        }
    }
}

pub fn state_key(guild_id: Id<GuildMarker>, voice_channel_id: Id<ChannelMarker>) -> (u64, u64) {
    (guild_id.get(), voice_channel_id.get())
}

pub fn find_vc_for_session(states: &RentalStateMap, session_id: i32) -> u64 {
    for entry in states.iter() {
        if entry.session_id() == session_id {
            return entry.key().1;
        }
    }
    0
}

