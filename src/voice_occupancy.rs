use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct VoiceOccupancy {
    users_by_channel: DashMap<(u64, u64), HashSet<u64>>,
    channel_by_user: DashMap<(u64, u64), u64>,
}

pub type VoiceOccupancyMap = Arc<VoiceOccupancy>;

pub fn new_voice_occupancy_map() -> VoiceOccupancyMap {
    Arc::new(VoiceOccupancy::default())
}

impl VoiceOccupancy {
    pub fn add_user(&self, guild_id: u64, user_id: u64, channel_id: u64) {
        if let Some(previous_channel_id) =
            self.channel_by_user.insert((guild_id, user_id), channel_id)
            && previous_channel_id != channel_id
        {
            self.remove_user_from_channel(guild_id, user_id, previous_channel_id);
        }

        self.users_by_channel
            .entry((guild_id, channel_id))
            .or_default()
            .insert(user_id);
    }

    pub fn remove_user(&self, guild_id: u64, user_id: u64, channel_id: u64) {
        self.channel_by_user.remove(&(guild_id, user_id));
        self.remove_user_from_channel(guild_id, user_id, channel_id);
    }

    fn remove_user_from_channel(&self, guild_id: u64, user_id: u64, channel_id: u64) {
        let key = (guild_id, channel_id);
        let is_empty = if let Some(mut users) = self.users_by_channel.get_mut(&key) {
            users.remove(&user_id);
            users.is_empty()
        } else {
            false
        };

        if is_empty {
            self.users_by_channel.remove(&key);
        }
    }

    pub fn clear_guild(&self, guild_id: u64) {
        self.users_by_channel
            .retain(|(entry_guild_id, _), _| *entry_guild_id != guild_id);
        self.channel_by_user
            .retain(|(entry_guild_id, _), _| *entry_guild_id != guild_id);
    }

    pub fn channel_for_user(&self, guild_id: u64, user_id: u64) -> Option<u64> {
        self.channel_by_user
            .get(&(guild_id, user_id))
            .map(|channel_id| *channel_id)
    }

    pub fn has_users(&self, guild_id: u64, channel_id: u64) -> bool {
        self.users_by_channel
            .get(&(guild_id, channel_id))
            .is_some_and(|users| !users.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::new_voice_occupancy_map;

    #[test]
    fn tracks_channel_occupancy() {
        let occupancy = new_voice_occupancy_map();

        assert!(!occupancy.has_users(1, 10));

        occupancy.add_user(1, 100, 10);

        assert_eq!(occupancy.channel_for_user(1, 100), Some(10));
        assert!(occupancy.has_users(1, 10));

        occupancy.remove_user(1, 100, 10);

        assert_eq!(occupancy.channel_for_user(1, 100), None);
        assert!(!occupancy.has_users(1, 10));
    }

    #[test]
    fn keeps_channel_occupied_until_last_user_leaves() {
        let occupancy = new_voice_occupancy_map();

        occupancy.add_user(1, 100, 10);
        occupancy.add_user(1, 200, 10);
        occupancy.remove_user(1, 100, 10);

        assert!(occupancy.has_users(1, 10));

        occupancy.remove_user(1, 200, 10);

        assert!(!occupancy.has_users(1, 10));
    }

    #[test]
    fn moving_user_updates_old_and_new_channel() {
        let occupancy = new_voice_occupancy_map();

        occupancy.add_user(1, 100, 10);
        occupancy.add_user(1, 100, 20);

        assert!(!occupancy.has_users(1, 10));
        assert!(occupancy.has_users(1, 20));
        assert_eq!(occupancy.channel_for_user(1, 100), Some(20));
    }

    #[test]
    fn clears_guild_occupancy() {
        let occupancy = new_voice_occupancy_map();

        occupancy.add_user(1, 100, 10);
        occupancy.add_user(2, 200, 20);
        occupancy.clear_guild(1);

        assert!(!occupancy.has_users(1, 10));
        assert_eq!(occupancy.channel_for_user(1, 100), None);
        assert!(occupancy.has_users(2, 20));
        assert_eq!(occupancy.channel_for_user(2, 200), Some(20));
    }
}
