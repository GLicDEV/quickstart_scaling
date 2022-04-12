use crate::{Principal, TimestampMillis};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

//Business State
#[derive(CandidType, Deserialize, Debug)]
pub struct BusinessState {
    entries: HashMap<String, Vec<BucketEntry>>,
    current_entries: u64,
    bucket_max_entries: u64,
    content_moderators: Vec<Principal>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct BucketEntry {
    pub(crate) tag: String,
    pub(crate) body: String,
    pub(crate) submitted_at: TimestampMillis,
    pub(crate) submitted_by: Principal,
}

#[derive(CandidType, Default, Deserialize, Clone, Debug)]
pub struct BucketIndex {
    pub(crate) effective_index: EffectiveIndex,
    pub(crate) index_state: IndexState,
    pub(crate) last_updated: TimestampMillis,
}

#[derive(CandidType, Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct EffectiveIndex {
    tags: Vec<String>,
    current_entries: u64,
    bucket_max_entries: u64,
}

impl Default for BucketEntry {
    fn default() -> Self {
        BucketEntry {
            tag: "".to_string(),
            body: "".to_string(),
            submitted_at: 0,
            submitted_by: Principal::anonymous(),
        }
    }
}

impl Default for BusinessState {
    fn default() -> Self {
        BusinessState {
            entries: Default::default(),
            current_entries: 0,
            bucket_max_entries: 20,
            content_moderators: vec![],
        }
    }
}

#[derive(CandidType, Deserialize, Clone, Copy, Debug)]
pub enum IndexState {
    New,
    InSync(u32),
    Synced,
}

impl Default for IndexState {
    fn default() -> Self {
        IndexState::New
    }
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct BucketMetrics {
    pub(crate) canister_id: Principal,
    pub(crate) cycles_balance: u128,
    pub(crate) controllers: Vec<Principal>,
    pub(crate) moderators: Vec<Principal>,
    pub(crate) index_canister_id: Principal,
    pub(crate) max_entries: u64,
    pub(crate) current_entries: u64,
    pub(crate) memory_used: u64,
}

// This is the section that implements all our business logic, on top
// of the business state.
#[allow(dead_code)]
impl BusinessState {
    pub fn max_entries(&self) -> u64 {
        self.bucket_max_entries
    }

    pub fn set_max_entries(&mut self, max_entries: u64) {
        self.bucket_max_entries = max_entries;
    }

    pub fn entries_count(&self) -> u64 {
        self.current_entries
    }

    pub fn update_entries_count(&mut self, count: u64) {
        self.current_entries = count;
    }

    pub fn add_entry(&mut self, entry: BucketEntry) -> bool {
        if self.entries_count() < self.max_entries() {
            let key = entry.tag.clone();
            self.entries.entry(key).or_default().push(entry);

            //Don't forget to increase the entries counter
            //This bug was caught with the unit tests in "fn test_capacity()"
            //Comment the next line to see the test fail
            self.current_entries += 1;
            return true;
        }
        false
    }

    //List entries if they were submitted by a principal or by anonymous
    pub fn list_entries(&self, tag: &str, submitted_by: Principal) -> Vec<BucketEntry> {
        let mut filtered_entries: Vec<BucketEntry> = Vec::new();

        for v in self.entries.get(tag) {
            for i in v.into_iter() {
                if i.submitted_by == submitted_by || i.submitted_by == Principal::anonymous() {
                    filtered_entries.push(i.clone())
                }
            }
        }

        filtered_entries
    }

    pub fn list_all_entries(&self) -> Vec<BucketEntry> {
        let mut all_entries: Vec<BucketEntry> = Vec::new();
        let all_keys = self.entries.keys().into_iter();

        for key in all_keys {
            for v in self.entries.get(key) {
                for i in v.into_iter() {
                    all_entries.push(i.clone())
                }
            }
        }

        all_entries
    }

    pub fn create_bucket_index(&self) -> EffectiveIndex {
        let all_keys = self.entries.keys().map(|s| s.clone()).collect();

        EffectiveIndex {
            tags: all_keys,
            current_entries: self.current_entries,
            bucket_max_entries: self.bucket_max_entries,
        }
    }

    pub fn add_content_moderator(&mut self, moderator: Principal) {
        self.content_moderators.push(moderator);
    }

    pub fn add_content_moderators(&mut self, moderators: Vec<Principal>) {
        self.content_moderators = moderators;
    }

    pub fn get_content_moderators(&self) -> Vec<Principal> {
        self.content_moderators.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let mut business_state = BusinessState::default();

        println!("{:?}", business_state);
        assert_eq!(business_state.max_entries(), 20);
        assert_eq!(business_state.entries_count(), 0);

        business_state.set_max_entries(40);
        assert_eq!(business_state.max_entries(), 40);
    }

    #[test]
    fn test_insert() {
        let mut business_state = BusinessState::default();
        business_state.set_max_entries(10);

        let entry = BucketEntry {
            tag: "#rabbit".to_string(),
            body: "Rabbits are fluffy animals".to_string(),
            submitted_at: 0,
            submitted_by: Principal::anonymous(),
        };

        let res = business_state.add_entry(entry.clone());
        assert_eq!(res, true);
        assert_eq!(business_state.entries_count(), 1);

        let res = business_state.add_entry(entry.clone());
        assert_eq!(res, true);
        assert_eq!(business_state.entries_count(), 2);
    }

    #[test]
    fn test_capacity() {
        let mut business_state = BusinessState::default();

        let entry = BucketEntry {
            tag: "#rabbit".to_string(),
            body: "Rabbits are fluffy animals".to_string(),
            submitted_at: 0,
            submitted_by: Principal::anonymous(),
        };

        business_state.set_max_entries(3);

        let res = business_state.add_entry(entry.clone());
        assert_eq!(res, true);
        let res = business_state.add_entry(entry.clone());
        assert_eq!(res, true);
        let res = business_state.add_entry(entry.clone());
        assert_eq!(res, true);
        let res = business_state.add_entry(entry.clone());
        assert_eq!(res, false);

        assert_eq!(business_state.current_entries, 3);
    }

    #[test]
    fn test_filter() {
        let user1: Principal = Principal::from_slice(&[1]);
        let user2: Principal = Principal::from_slice(&[2]);

        let mut business_state = BusinessState::default();
        business_state.set_max_entries(10);

        let entry = BucketEntry {
            tag: "#rabbit".to_string(),
            body: "Rabbits are fluffy animals".to_string(),
            submitted_at: 0,
            submitted_by: user1,
        };

        let _res = business_state.add_entry(entry.clone());

        let entry = BucketEntry {
            tag: "#rabbit".to_string(),
            body: "Rabbits are fluffy animals".to_string(),
            submitted_at: 0,
            submitted_by: user2,
        };

        let _res = business_state.add_entry(entry.clone());

        assert_eq!(business_state.list_all_entries().len(), 2);
        assert_eq!(business_state.list_entries("#rabbit", user1).len(), 1);
        assert_eq!(business_state.list_entries("#rabbit", user2).len(), 1);

        assert_eq!(business_state.list_entries("#dog", user1).len(), 0);
        assert_eq!(business_state.list_entries("#fox", user2).len(), 0);
    }

    #[test]
    fn test_print() {
        let mut business_state = BusinessState::default();

        business_state.set_max_entries(3);

        let entry = BucketEntry {
            tag: "#rabbit".to_string(),
            body: "Rabbits are fluffy animals".to_string(),
            submitted_at: 0,
            submitted_by: Principal::anonymous(),
        };

        let _res = business_state.add_entry(entry.clone());

        let entry = BucketEntry {
            tag: "#rabbit".to_string(),
            body: "Rabbits are cute animals".to_string(),
            submitted_at: 0,
            submitted_by: Principal::anonymous(),
        };
        let _res = business_state.add_entry(entry.clone());

        let entry = BucketEntry {
            tag: "#rabbit".to_string(),
            body: "Rabbits are cute and fluffy animals".to_string(),
            submitted_at: 0,
            submitted_by: Principal::anonymous(),
        };
        let _res = business_state.add_entry(entry.clone());

        let all_entries = business_state.list_all_entries();

        println!("{:?}", all_entries);
    }
}
