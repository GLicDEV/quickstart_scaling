use crate::businesslogic::IndexingStrategy::BalancedLoad;
use crate::{Principal, RuntimeState, TimestampMillis, RUNTIME_STATE};
use candid::{CandidType, Encode, Nat};
use ic_cdk::print;
use serde::{Deserialize, Serialize};
use std::cell::{Ref, RefMut};
use std::cmp::Reverse;
use std::collections::HashMap;

//Business State
#[derive(CandidType, Deserialize, Debug, Default)]
pub struct BusinessState {
    bucket_indexes: HashMap<Principal, EffectiveIndex>,
    spawned_buckets: Vec<SpawnedBucketCanister>,
    pub(crate) global_index: GlobalIndex,
    current_buckets_free_slots: u128,
    pub(crate) planned_buckets: Vec<PlannedBucketCanister>,
    indexing_strategy: IndexingStrategy,
    content_moderators: Vec<Principal>,
    pub(crate) push_moderators: bool,
}

#[derive(CandidType, Deserialize, Debug, Default, Clone)]
pub struct GlobalIndex {
    pub(crate) tag_to_canisters: HashMap<String, Vec<Principal>>,
    pub(crate) last_updated: TimestampMillis,
}

#[derive(CandidType, Default, Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct EffectiveIndex {
    tags: Vec<String>,
    current_entries: u64,
    bucket_max_entries: u64,
}

#[derive(CandidType, Debug, Deserialize)]
pub enum IndexingStrategy {
    BalancedLoad,
    FillFirst,
}

impl Default for IndexingStrategy {
    fn default() -> Self {
        BalancedLoad
    }
}

#[derive(CandidType, Deserialize, Debug, Default)]
pub struct SpawnedBucketCanister {}

#[derive(CandidType, Deserialize, Debug, Default)]
pub struct BucketCanisterSettings {}

#[derive(CandidType, Deserialize, Debug)]
pub enum SpawnStatus {
    New,
    InWork(u32),
    Installed,
}

impl Default for SpawnStatus {
    fn default() -> Self {
        SpawnStatus::New
    }
}

#[derive(CandidType, Deserialize, Debug)]
pub struct PlannedBucketCanister {
    pub(crate) canister_settings: BucketCanisterSettings,
    pub(crate) spawn_status: SpawnStatus,
    pub(crate) bucket_max_entries: u128,
}

impl Default for PlannedBucketCanister {
    fn default() -> Self {
        PlannedBucketCanister {
            canister_settings: Default::default(),
            spawn_status: Default::default(),
            bucket_max_entries: 20,
        }
    }
}

// This way of importing creates a larger index canister binary. We could upload the bucket wasm
// bytes after the installation, in a config update call. This would create a smaller
// Index binary but it would require a two step process in installing the canister.
const BUCKET_WASM: &[u8] = std::include_bytes!(
    "../../../target/wasm32-unknown-unknown/release/quickstart_scaling_bucket-opt.wasm"
);

// This is the section that implements all our business logic, on top
// of the business state.
#[allow(dead_code)]
impl BusinessState {
    pub fn where_to_upload(&self) -> Vec<Principal> {
        let mut free_slot_list: Vec<(Principal, u128)> = self
            .bucket_indexes
            .iter()
            .filter(|(_k, v)| v.current_entries != v.bucket_max_entries)
            .map(|(k, v)| (k.clone(), v.current_entries as u128))
            .collect();

        match self.indexing_strategy {
            IndexingStrategy::BalancedLoad => {
                free_slot_list.sort_by_key(|k| k.1);
            }
            IndexingStrategy::FillFirst => {
                free_slot_list.sort_by_key(|k| Reverse(k.1));
            }
        }
        let can_list: Vec<Principal> = free_slot_list.iter().map(|s| s.0).collect();
        can_list
    }

    pub fn set_indexing_strategy(&mut self, strategy: IndexingStrategy) {
        self.indexing_strategy = strategy;
    }

    pub fn add_bucket_index(&mut self, canister_id: Principal, effective_index: EffectiveIndex) {
        self.bucket_indexes.insert(canister_id, effective_index);

        // Once we receive a new bucket index, we should update the free slots
        self.update_free_slots()
    }

    pub fn add_spawned_bucket(&mut self, spawned_bucket: SpawnedBucketCanister) {
        self.spawned_buckets.push(spawned_bucket);
    }

    pub fn generate_index_tag_to_canisters(&self) -> HashMap<String, Vec<Principal>> {
        let mut tag2can: HashMap<String, Vec<Principal>> = Default::default();

        for (canister_id, effective_index) in self.bucket_indexes.iter() {
            for tag in effective_index.tags.iter() {
                tag2can
                    .entry(tag.clone())
                    .or_default()
                    .push(canister_id.clone());
            }
        }

        tag2can
    }

    pub fn get_index_by_tag(&self, tag: &str) -> Vec<Principal> {
        self.global_index
            .tag_to_canisters
            .get(tag)
            .unwrap_or(&vec![])
            .clone()
    }

    pub fn get_all_buckets(&self) -> Vec<Principal> {
        self.bucket_indexes.keys().map(|key| key.clone()).collect()
    }

    pub fn get_global_index(&self) -> GlobalIndex {
        self.global_index.clone()
    }

    // We can't send a hashmap<String, Vec<Principal>> as candid encoded data,
    // so we convert everything to strings. This will only be used for demo
    // purposes, so we should be fine here. This is not needed for business logic.
    pub fn get_index_tag2can_as_vec(&self) -> Vec<Vec<String>> {
        let mut t2c_vec: Vec<Vec<String>> = vec![];

        for (key, value) in self.global_index.tag_to_canisters.iter() {
            let mut row: Vec<String> = vec![];
            row.push(key.clone());
            for p in value.iter() {
                row.push(p.clone().to_string())
            }
            t2c_vec.push(row);
        }

        t2c_vec
    }

    pub fn calculate_free_slots(&self) -> u128 {
        // let mut free_slots: u128 = 0;

        // for (_, effective_index) in self.bucket_indexes.iter() {
        //     free_slots +=
        //         (effective_index.bucket_max_entries - effective_index.current_entries) as u128;
        // }
        let free_slots = self
            .bucket_indexes
            .iter()
            .map(|(_, index)| (index.bucket_max_entries - index.current_entries) as u128)
            .sum();

        free_slots
    }

    pub fn update_free_slots(&mut self) {
        self.current_buckets_free_slots = self.calculate_free_slots()
    }

    pub fn get_free_slots(&self) -> u128 {
        self.current_buckets_free_slots
    }

    pub fn get_planned_slots(&self) -> u128 {
        let planned_slots = self
            .planned_buckets
            .iter()
            .map(|canister| canister.bucket_max_entries)
            .sum();

        planned_slots
    }

    pub fn add_planned_bucket(&mut self) {
        let bucket = PlannedBucketCanister::default();

        self.planned_buckets.push(bucket);
    }

    pub fn add_content_moderator(&mut self, moderator: Principal) {
        self.content_moderators.push(moderator);
    }

    pub fn get_content_moderators(&self) -> Vec<Principal> {
        self.content_moderators.clone()
    }
}

// New bucket spawning, Inter canister communication and other canister 2 canister
// functionality. Most of this is used in heartbeat()
// This should probably be moved to a dedicated data structure & an impl block
// Might need to move some things like canister settings from "global" data

pub async fn spawn_bucket_loop() {
    // spawn a new bucket if there are any in the planned queue
    //
    // lock planned bucket
    let planned_bucket_lock: u32 = RUNTIME_STATE.with(|state| prep_lock_bucket(state.borrow_mut()));

    // 0 means we don't have any planned buckets to install
    if planned_bucket_lock > 0 {
        // prep canister create
        let canister_create_args =
            RUNTIME_STATE.with(|state| prep_canister_create(state.borrow_mut()));

        // call canister create
        let canister_id = call_canister_create(canister_create_args).await;

        // call_canister_create will return anonymous if it can't create a bucket
        if canister_id != Principal::anonymous() {
            print(format!("Created canister: {}", canister_id.to_text()));

            // prep canister install
            let canister_install_args = Encode!(&CanisterInstallSendArgs {
                greet: "Hello from Index".to_string(),
                controllers: vec![Principal::from_text(
                    "l6s27-7ndcl-nowe5-xeyf7-ymdnq-dkemz-jkhfw-zr5wu-jvf2p-aupzq-2qe",
                )
                .unwrap(),],
            })
            .unwrap();

            // call canister install
            let result: bool = call_canister_install(&canister_id, canister_install_args).await;
            print(format!("Cannister install: {}", result));

            if result {
                // set planned bucket to installed
                RUNTIME_STATE.with(|state| {
                    update_planned_bucket(true, planned_bucket_lock, state.borrow_mut())
                });
            } else {
                RUNTIME_STATE.with(|state| {
                    update_planned_bucket(false, planned_bucket_lock, state.borrow_mut())
                });
            }
        }
    }
}

fn update_planned_bucket(
    installed: bool,
    planned_bucket_lock: u32,
    mut runtime_state: RefMut<RuntimeState>,
) {
    for bucket in runtime_state.data.business_state.planned_buckets.iter_mut() {
        if let SpawnStatus::InWork(lock) = bucket.spawn_status {
            if lock == planned_bucket_lock {
                if installed {
                    bucket.spawn_status = SpawnStatus::Installed;
                    bucket.bucket_max_entries = 0;
                } else {
                    bucket.spawn_status = SpawnStatus::New;
                }
            }
        }
    }
}

async fn call_bucket_push_moderators(canister_id: Principal, moderators: Vec<Principal>) -> bool {
    match ic_cdk::api::call::call(canister_id, "add_content_moderators", (moderators,)).await {
        Ok(x) => x,
        Err((code, msg)) => {
            print(format!(
                "An error happened during the call: {}: {}",
                code as u8, msg
            ));
            return false;
        }
    };

    true
}

async fn call_canister_install(canister_id: &Principal, canister_install_args: Vec<u8>) -> bool {
    let install_config: CanisterInstall = CanisterInstall {
        mode: InstallMode::Install,
        canister_id: canister_id.clone(),
        wasm_module: BUCKET_WASM.to_vec(),
        arg: canister_install_args,
    };

    match ic_cdk::api::call::call(
        Principal::management_canister(),
        "install_code",
        (install_config,),
    )
    .await
    {
        Ok(x) => x,
        Err((code, msg)) => {
            print(format!(
                "An error happened during the call: {}: {}",
                code as u8, msg
            ));
            return false;
        }
    };

    true
}

async fn call_canister_create(canister_create_args: CreateCanisterArgs) -> Principal {
    print("creating bucket...");

    #[derive(CandidType)]
    struct In {
        settings: Option<CreateCanisterSettings>,
    }

    let in_arg = In {
        settings: Some(canister_create_args.settings),
    };

    let (create_result,): (CanisterIdRecord,) = match ic_cdk::api::call::call_with_payment(
        Principal::management_canister(),
        "create_canister",
        (in_arg,),
        canister_create_args.cycles,
    )
    .await
    {
        Ok(x) => x,
        Err((code, msg)) => {
            print(format!(
                "An error happened during the call: {}: {}",
                code as u8, msg
            ));

            (CanisterIdRecord {
                canister_id: Principal::anonymous(),
            },)
        }
    };

    // print(format!("{}", create_result.canister_id.to_text()));

    create_result.canister_id
}

fn prep_canister_create(runtime_state: RefMut<RuntimeState>) -> CreateCanisterArgs {
    let controller_id = runtime_state.env.canister_id();

    // Add your own principal as a controller, in case manual control is needed
    let create_args = CreateCanisterArgs {
        cycles: 100_000_000_000,
        settings: CreateCanisterSettings {
            controllers: Some(vec![
                controller_id.clone(),
                Principal::from_text(
                    "l6s27-7ndcl-nowe5-xeyf7-ymdnq-dkemz-jkhfw-zr5wu-jvf2p-aupzq-2qe",
                )
                .unwrap(),
            ]),
            compute_allocation: None,
            memory_allocation: None,
            freezing_threshold: None,
        },
    };

    create_args
}

fn prep_lock_bucket(mut runtime_state: RefMut<RuntimeState>) -> u32 {
    let bucket_lock = runtime_state.env.random_u32();

    for bucket in runtime_state.data.business_state.planned_buckets.iter_mut() {
        if let SpawnStatus::New = bucket.spawn_status {
            bucket.spawn_status = SpawnStatus::InWork(bucket_lock);
            return bucket_lock;
        }
    }
    0
}

pub(crate) fn should_spawn_buckets(runtime_state: Ref<RuntimeState>) -> bool {
    // This code looks fine at first glance but it may lead to generating lots
    // of buckets if it takes more than one heartbeat iteration to spawn a canister
    // This would get checked every heartbeat iteration and resolve to true
    //
    // runtime_state.data.canister_settings.desired_free_slots
    //     < runtime_state.data.business_state.get_free_slots()

    // We need to sum the available free slots with the planned free slots,
    // so that we don't add too many buckets
    runtime_state.data.business_state.get_planned_slots()
        + runtime_state.data.business_state.get_free_slots()
        < runtime_state.data.canister_settings.desired_free_slots
}

pub(crate) fn reindex_tag_to_canisters(mut runtime_state: RefMut<RuntimeState>) {
    if runtime_state.env.now() - runtime_state.data.business_state.global_index.last_updated
        > runtime_state.data.canister_settings.reindex_interval
    {
        // print(format!(
        //     "{:?}",
        //     runtime_state
        //         .data
        //         .business_state
        //         .generate_index_tag_to_canisters()
        // ));

        runtime_state
            .data
            .business_state
            .global_index
            .tag_to_canisters = runtime_state
            .data
            .business_state
            .generate_index_tag_to_canisters();

        runtime_state.data.business_state.global_index.last_updated = runtime_state.env.now();
    }
}

#[derive(CandidType, Deserialize)]
struct CanisterInstallSendArgs {
    greet: String,
    controllers: Vec<Principal>,
}

#[derive(CandidType, Clone, Deserialize, Debug)]
pub struct CanisterIdRecord {
    pub canister_id: Principal,
}

#[derive(CandidType, Debug, Clone, Deserialize)]
pub struct CreateCanisterSettings {
    pub controllers: Option<Vec<Principal>>,
    pub compute_allocation: Option<Nat>,
    pub memory_allocation: Option<Nat>,
    pub freezing_threshold: Option<Nat>,
}

#[derive(CandidType, Clone, Deserialize)]
pub struct CreateCanisterArgs {
    pub cycles: u64,
    pub settings: CreateCanisterSettings,
}

#[derive(CandidType, Deserialize)]
enum InstallMode {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "reinstall")]
    Reinstall,
    #[serde(rename = "upgrade")]
    Upgrade,
}

#[derive(CandidType, Deserialize)]
struct CanisterInstall {
    mode: InstallMode,
    canister_id: Principal,
    #[serde(with = "serde_bytes")]
    wasm_module: Vec<u8>,
    #[serde(with = "serde_bytes")]
    arg: Vec<u8>,
}

// Unit tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let business_state = BusinessState::default();

        assert_eq!(business_state.spawned_buckets.len(), 0);
        assert_eq!(business_state.global_index.tag_to_canisters.len(), 0);
        assert_eq!(business_state.bucket_indexes.len(), 0);
    }

    #[test]
    fn insert_bucket_indexes() {
        let mut business_state = BusinessState::default();

        let bucket_index1 = EffectiveIndex {
            tags: vec![
                "#rabbit".to_string(),
                "#fox".to_string(),
                "#dog".to_string(),
            ],
            current_entries: 5,
            bucket_max_entries: 20,
        };
        let can_id1 = Principal::from_slice(&[1]);

        business_state.add_bucket_index(can_id1, bucket_index1);

        let bucket_index2 = EffectiveIndex {
            tags: vec![
                "#rabbit".to_string(),
                "#cat".to_string(),
                "#fish".to_string(),
            ],
            current_entries: 3,
            bucket_max_entries: 20,
        };
        let can_id2 = Principal::from_slice(&[2]);

        business_state.add_bucket_index(can_id2, bucket_index2);

        assert_eq!(business_state.bucket_indexes.len(), 2);
    }

    #[test]
    fn gen_global_index_tag2can() {
        let mut business_state = BusinessState::default();

        let bucket_index1 = EffectiveIndex {
            tags: vec![
                "#rabbit".to_string(),
                "#fox".to_string(),
                "#dog".to_string(),
            ],
            current_entries: 5,
            bucket_max_entries: 20,
        };
        let can_id1 = Principal::from_slice(&[1]);

        business_state.add_bucket_index(can_id1, bucket_index1);

        let bucket_index2 = EffectiveIndex {
            tags: vec![
                "#rabbit".to_string(),
                "#cat".to_string(),
                "#fish".to_string(),
            ],
            current_entries: 3,
            bucket_max_entries: 20,
        };
        let can_id2 = Principal::from_slice(&[2]);

        business_state.add_bucket_index(can_id2, bucket_index2);

        let tag2can = business_state.generate_index_tag_to_canisters();

        assert_eq!(tag2can.get("#rabbit").unwrap().len(), 2);
        assert_eq!(tag2can.get("#fox").unwrap().len(), 1);
        assert_eq!(tag2can.get("#none").unwrap_or(&vec![]).len(), 0);

        println!("{:?}", tag2can);
    }

    #[test]
    fn upload_strategy() {
        let mut business_state = BusinessState::default();

        let bucket_index1 = EffectiveIndex {
            tags: vec![
                "#rabbit".to_string(),
                "#fox".to_string(),
                "#dog".to_string(),
            ],
            current_entries: 5,
            bucket_max_entries: 20,
        };
        let can_id1 = Principal::from_slice(&[1]);

        business_state.add_bucket_index(can_id1, bucket_index1.clone());

        let bucket_index2 = EffectiveIndex {
            tags: vec![
                "#rabbit".to_string(),
                "#cat".to_string(),
                "#fish".to_string(),
            ],
            current_entries: 3,
            bucket_max_entries: 20,
        };
        let can_id2 = Principal::from_slice(&[2]);

        business_state.add_bucket_index(can_id2, bucket_index2.clone());

        let bucket_index3 = EffectiveIndex {
            tags: vec![
                "#rabbit".to_string(),
                "#cat".to_string(),
                "#fish".to_string(),
            ],
            current_entries: 6,
            bucket_max_entries: 20,
        };
        let can_id3 = Principal::from_slice(&[3]);

        business_state.add_bucket_index(can_id3, bucket_index3.clone());

        // uuc56-gyb:5 hqgi5-iic:3 jmf34-nyd:6

        business_state.indexing_strategy = IndexingStrategy::BalancedLoad;

        // 3 5 6 -> we're filling from most empty when in BalancedLoad
        assert_eq!(
            business_state.where_to_upload(),
            vec![
                Principal::from_text("hqgi5-iic").unwrap(),
                Principal::from_text("uuc56-gyb").unwrap(),
                Principal::from_text("jmf34-nyd").unwrap(),
            ]
        );

        business_state.indexing_strategy = IndexingStrategy::FillFirst;

        // 6 5 3 -> we're filling from most entries first, in FillFirst
        assert_eq!(
            business_state.where_to_upload(),
            vec![
                Principal::from_text("jmf34-nyd").unwrap(),
                Principal::from_text("uuc56-gyb").unwrap(),
                Principal::from_text("hqgi5-iic").unwrap(),
            ]
        );

        println!(
            "{}:{} {}:{} {}:{}",
            can_id1.to_text(),
            bucket_index1.current_entries,
            can_id2.to_text(),
            bucket_index2.current_entries,
            can_id3.to_text(),
            bucket_index3.current_entries
        );

        println!(
            "Indexing strategy: {:?} {:?}",
            business_state.indexing_strategy,
            business_state
                .where_to_upload()
                .iter()
                .map(|s| s.to_text())
                .collect::<Vec<String>>()
        );
    }
}

pub(crate) async fn push_moderators() {
    // Check if we need to push moderators first
    if let true = RUNTIME_STATE.with(|state| state.borrow().data.business_state.push_moderators) {
        let prep_push = RUNTIME_STATE.with(|state| prep_push_moderators(state.borrow()));

        for canister_id in prep_push.0 {
            let _result =
                call_bucket_push_moderators(canister_id.clone(), prep_push.1.clone()).await;
        }

        // Unset push_moderators
        RUNTIME_STATE.with(|state| state.borrow_mut().data.business_state.push_moderators = false);
    }
}

fn prep_push_moderators(runtime_state: Ref<RuntimeState>) -> (Vec<Principal>, Vec<Principal>) {
    (
        runtime_state.data.business_state.get_all_buckets(),
        runtime_state.data.business_state.get_content_moderators(),
    )
}
