// ABOUTME: Group management for multi-room audio
// ABOUTME: Handles grouping of clients for synchronized playback

use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Playback state of a group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    /// Not playing anything
    Stopped,
    /// Currently playing
    Playing,
    /// Paused
    Paused,
}

impl PlaybackState {
    /// Convert to protocol string
    pub fn as_str(&self) -> &'static str {
        match self {
            PlaybackState::Stopped => "stopped",
            PlaybackState::Playing => "playing",
            PlaybackState::Paused => "paused",
        }
    }
}

/// A group of synchronized clients
#[derive(Debug)]
pub struct Group {
    /// Unique group identifier
    pub id: String,
    /// Human-readable group name
    pub name: String,
    /// Client IDs in this group
    pub members: HashSet<String>,
    /// Current playback state
    pub playback_state: PlaybackState,
    /// Group volume (0-100)
    pub volume: u8,
    /// Group mute state
    pub muted: bool,
}

impl Group {
    /// Create a new group
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            members: HashSet::new(),
            playback_state: PlaybackState::Stopped,
            volume: 100,
            muted: false,
        }
    }

    /// Add a client to the group
    pub fn add_member(&mut self, client_id: String) {
        self.members.insert(client_id);
    }

    /// Remove a client from the group
    pub fn remove_member(&mut self, client_id: &str) -> bool {
        self.members.remove(client_id)
    }

    /// Check if a client is in this group
    pub fn has_member(&self, client_id: &str) -> bool {
        self.members.contains(client_id)
    }

    /// Get the number of members
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Check if the group is empty
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

/// Manages all groups
#[derive(Debug)]
pub struct GroupManager {
    /// Map of group_id to group
    groups: Arc<RwLock<HashMap<String, Group>>>,
    /// Default group ID
    default_group_id: String,
}

impl GroupManager {
    /// Create a new group manager with a default group
    pub fn new() -> Self {
        let default_id = "default".to_string();
        let default_group = Group::new(&default_id, "Default Group");

        let mut groups = HashMap::new();
        groups.insert(default_id.clone(), default_group);

        Self {
            groups: Arc::new(RwLock::new(groups)),
            default_group_id: default_id,
        }
    }

    /// Get the default group ID
    pub fn default_group_id(&self) -> &str {
        &self.default_group_id
    }

    /// Create a new group
    pub fn create_group(&self, id: impl Into<String>, name: impl Into<String>) -> String {
        let id = id.into();
        let group = Group::new(&id, name);
        self.groups.write().insert(id.clone(), group);
        id
    }

    /// Delete a group (members will be moved to default group)
    pub fn delete_group(&self, group_id: &str) -> Vec<String> {
        if group_id == self.default_group_id {
            return Vec::new(); // Can't delete default group
        }

        let mut groups = self.groups.write();
        if let Some(group) = groups.remove(group_id) {
            let members: Vec<_> = group.members.into_iter().collect();

            // Move members to default group
            if let Some(default) = groups.get_mut(&self.default_group_id) {
                for member in &members {
                    default.add_member(member.clone());
                }
            }

            members
        } else {
            Vec::new()
        }
    }

    /// Add a client to a group
    pub fn add_to_group(&self, client_id: &str, group_id: &str) -> bool {
        let mut groups = self.groups.write();

        // Remove from current group first
        for group in groups.values_mut() {
            group.remove_member(client_id);
        }

        // Add to new group
        if let Some(group) = groups.get_mut(group_id) {
            group.add_member(client_id.to_string());
            true
        } else {
            // Group doesn't exist, add to default
            if let Some(default) = groups.get_mut(&self.default_group_id) {
                default.add_member(client_id.to_string());
            }
            false
        }
    }

    /// Remove a client from all groups
    pub fn remove_client(&self, client_id: &str) {
        let mut groups = self.groups.write();
        for group in groups.values_mut() {
            group.remove_member(client_id);
        }
    }

    /// Get the group ID for a client
    pub fn get_client_group(&self, client_id: &str) -> Option<String> {
        let groups = self.groups.read();
        for (id, group) in groups.iter() {
            if group.has_member(client_id) {
                return Some(id.clone());
            }
        }
        None
    }

    /// Get group info by ID
    pub fn get_group(&self, group_id: &str) -> Option<(String, String, PlaybackState)> {
        let groups = self.groups.read();
        groups.get(group_id).map(|g| {
            (g.id.clone(), g.name.clone(), g.playback_state)
        })
    }

    /// Set playback state for a group
    pub fn set_playback_state(&self, group_id: &str, state: PlaybackState) {
        if let Some(group) = self.groups.write().get_mut(group_id) {
            group.playback_state = state;
        }
    }

    /// Get playback state for a group
    pub fn get_playback_state(&self, group_id: &str) -> Option<PlaybackState> {
        self.groups.read().get(group_id).map(|g| g.playback_state)
    }

    /// Set volume for a group
    pub fn set_volume(&self, group_id: &str, volume: u8) {
        if let Some(group) = self.groups.write().get_mut(group_id) {
            group.volume = volume.min(100);
        }
    }

    /// Set mute state for a group
    pub fn set_muted(&self, group_id: &str, muted: bool) {
        if let Some(group) = self.groups.write().get_mut(group_id) {
            group.muted = muted;
        }
    }

    /// Get all members of a group
    pub fn get_group_members(&self, group_id: &str) -> Vec<String> {
        self.groups
            .read()
            .get(group_id)
            .map(|g| g.members.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all group IDs
    pub fn group_ids(&self) -> Vec<String> {
        self.groups.read().keys().cloned().collect()
    }
}

impl Default for GroupManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GroupManager {
    fn clone(&self) -> Self {
        Self {
            groups: Arc::clone(&self.groups),
            default_group_id: self.default_group_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_basic() {
        let mut group = Group::new("test", "Test Group");
        assert!(group.is_empty());

        group.add_member("client1".to_string());
        assert_eq!(group.member_count(), 1);
        assert!(group.has_member("client1"));

        group.remove_member("client1");
        assert!(group.is_empty());
    }

    #[test]
    fn test_group_manager() {
        let manager = GroupManager::new();

        // Default group should exist
        assert!(manager.get_group("default").is_some());

        // Add client to default group
        manager.add_to_group("client1", "default");
        assert_eq!(manager.get_client_group("client1"), Some("default".to_string()));

        // Create new group and move client
        manager.create_group("room1", "Living Room");
        manager.add_to_group("client1", "room1");
        assert_eq!(manager.get_client_group("client1"), Some("room1".to_string()));

        // Remove client
        manager.remove_client("client1");
        assert_eq!(manager.get_client_group("client1"), None);
    }
}
