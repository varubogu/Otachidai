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
        prompt_message: Option<RentalPromptMessage>,
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

#[derive(Debug, Clone, Copy)]
pub struct RentalPromptMessage {
    pub channel_id: u64,
    pub message_id: u64,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_handle() -> JoinHandle<()> {
        tokio::spawn(std::future::pending())
    }

    #[tokio::test]
    async fn session_id_extracted_from_each_state() {
        let awaiting = RentalStateEntry {
            state: RentalState::AwaitingPurpose {
                session_id: 11,
                host_user_id: 1,
                timeout_task: dummy_handle(),
                prompt_message: None,
            },
            room_id: 1,
        };
        let active = RentalStateEntry {
            state: RentalState::Active {
                session_id: 22,
                host_user_id: 1,
            },
            room_id: 1,
        };
        let handoff = RentalStateEntry {
            state: RentalState::PendingHandoff {
                session_id: 33,
                timeout_task: dummy_handle(),
            },
            room_id: 1,
        };
        assert_eq!(awaiting.session_id(), 11);
        assert_eq!(active.session_id(), 22);
        assert_eq!(handoff.session_id(), 33);
    }

    #[test]
    fn state_key_uses_raw_ids() {
        let guild = Id::<GuildMarker>::new(123);
        let channel = Id::<ChannelMarker>::new(456);
        assert_eq!(state_key(guild, channel), (123, 456));
    }

    #[tokio::test]
    async fn find_vc_for_session_returns_matching_vc_then_zero() {
        let map = new_state_map();
        map.insert(
            (10, 999),
            RentalStateEntry {
                state: RentalState::Active {
                    session_id: 42,
                    host_user_id: 1,
                },
                room_id: 5,
            },
        );
        assert_eq!(find_vc_for_session(&map, 42), 999);
        // 見つからない場合は 0 を返す。
        assert_eq!(find_vc_for_session(&map, 7), 0);
    }

    #[tokio::test]
    async fn abort_timeout_cancels_pending_handoff_task() {
        let entry = RentalStateEntry {
            state: RentalState::PendingHandoff {
                session_id: 7,
                timeout_task: dummy_handle(),
            },
            room_id: 1,
        };
        entry.abort_timeout();
        let RentalStateEntry {
            state: RentalState::PendingHandoff { timeout_task, .. },
            ..
        } = entry
        else {
            unreachable!()
        };
        let err = timeout_task.await.unwrap_err();
        assert!(err.is_cancelled(), "abort 後のタスクはキャンセル扱いになる");
    }

    #[tokio::test]
    async fn abort_timeout_is_noop_for_active() {
        let entry = RentalStateEntry {
            state: RentalState::Active {
                session_id: 9,
                host_user_id: 1,
            },
            room_id: 1,
        };
        // Active にはタイムアウトタスクが無いので、呼んでも何も起きない（panic しない）。
        entry.abort_timeout();
    }
}
