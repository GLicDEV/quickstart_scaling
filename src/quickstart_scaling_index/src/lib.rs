mod businesslogic;
mod env;
mod lifetime;

use crate::businesslogic::{BusinessState, EffectiveIndex};
use crate::env::{CanisterEnv, EmptyEnv, Environment, TimestampMillis};
use ic_cdk::export::candid::{CandidType, Principal};
use ic_cdk_macros::*;
use serde::Deserialize;

use std::cell::{Ref, RefCell, RefMut};

#[derive(Debug)]
pub struct Error {
    pub code: u8,
    pub msg: String,
}

thread_local! {
    static RUNTIME_STATE: RefCell<RuntimeState> = RefCell::default();
}

struct RuntimeState {
    pub env: Box<dyn Environment>,
    pub data: Data,
}

impl Default for RuntimeState {
    fn default() -> Self {
        RuntimeState {
            env: Box::new(EmptyEnv {}),
            data: Data::default(),
        }
    }
}

#[derive(CandidType, Deserialize)]
struct IndexCanisterSettings {
    reindex_interval: TimestampMillis,
    desired_free_slots: u128,
}

impl Default for IndexCanisterSettings {
    fn default() -> Self {
        IndexCanisterSettings {
            // 5 seconds
            reindex_interval: 5_000_000_000,
            desired_free_slots: 60,
        }
    }
}

#[derive(CandidType, Default, Deserialize)]
struct Data {
    canister_settings: IndexCanisterSettings,
    business_state: BusinessState,
}

// MAIN FUNCTIONALITY
// Inter canister calls are named with snake case
#[update(name = "add_bucket_index")]
fn add_bucket_index(bucket_index: EffectiveIndex) -> bool {
    RUNTIME_STATE.with(|state| add_bucket_index_impl(bucket_index, state.borrow_mut()))
}

fn add_bucket_index_impl(
    bucket_index: EffectiveIndex,
    mut runtime_state: RefMut<RuntimeState>,
) -> bool {
    let caller = runtime_state.env.caller();
    runtime_state
        .data
        .business_state
        .add_bucket_index(caller, bucket_index);

    true
}

// Client facing calls are camelCase
// getMetrics is used for demo purposes
#[query(name = "getMetrics")]
fn get_metrics() -> String {
    RUNTIME_STATE.with(|state| get_metrics_impl(state.borrow()))
}

fn get_metrics_impl(runtime_state: Ref<RuntimeState>) -> String {
    format!(
        "CanisterID: {}\n
Cycles: {}\n
Free Slots: {}\n
Desired Free Slots: {}\n
Planned Slots: {}\n
All Buckets: {:?}\n
Memory: {}\n
Caller: {}\n",
        runtime_state.env.canister_id(),
        runtime_state.env.cycles_balance(),
        runtime_state.data.business_state.get_free_slots(),
        runtime_state.data.canister_settings.desired_free_slots,
        runtime_state.data.business_state.get_planned_slots(),
        runtime_state
            .data
            .business_state
            .get_all_buckets()
            .iter()
            .map(|b| b.to_text())
            .collect::<Vec<String>>(),
        runtime_state.env.memory_used(),
        runtime_state.env.caller().to_text(),
    )
}

// Just for demo purposes
#[query(name = "getGlobalIndex")]
fn get_global_index() -> Vec<Vec<String>> {
    RUNTIME_STATE.with(|state| get_global_index_impl(state.borrow()))
}

fn get_global_index_impl(runtime_state: Ref<RuntimeState>) -> Vec<Vec<String>> {
    runtime_state.data.business_state.get_index_tag2can_as_vec()
}

// Main call used by a client to get a list of buckets where it can find the
// data related to a #tag
#[query(name = "getIndexByTag")]
fn get_index_by_tag(tag: String) -> Vec<Principal> {
    RUNTIME_STATE.with(|state| get_index_by_tag_impl(tag, state.borrow()))
}

fn get_index_by_tag_impl(tag: String, runtime_state: Ref<RuntimeState>) -> Vec<Principal> {
    runtime_state.data.business_state.get_index_by_tag(&tag)
}

// Useful for demo purposes; could also be used by a client to "randomly" upload data
// to any canister, if this is something that works for their case.
#[query(name = "getAllIndexes")]
fn get_all_indexes() -> Vec<Principal> {
    RUNTIME_STATE.with(|state| get_all_indexes_impl(state.borrow()))
}

fn get_all_indexes_impl(runtime_state: Ref<RuntimeState>) -> Vec<Principal> {
    runtime_state.data.business_state.get_all_buckets()
}

#[query(name = "getUploadOrder")]
fn get_upload_order() -> Vec<Principal> {
    RUNTIME_STATE.with(|state| get_upload_order_impl(state.borrow()))
}

fn get_upload_order_impl(runtime_state: Ref<RuntimeState>) -> Vec<Principal> {
    runtime_state.data.business_state.where_to_upload()
}

#[update(name = "addContentModerator")]
fn add_content_moderator(moderator: Principal) {
    RUNTIME_STATE.with(|state| add_content_moderator_impl(moderator, state.borrow_mut()))
}

fn add_content_moderator_impl(moderator: Principal, mut runtime_state: RefMut<RuntimeState>) {
    runtime_state
        .data
        .business_state
        .add_content_moderator(moderator);

    // Every time we get a new moderator we push the vec to all the buckets
    // This can obviously be optimized based on the app's needs
    runtime_state.data.business_state.push_moderators = true;
}

// Make sure we can accept cycles from Bucket canisters
#[update]
fn wallet_receive() -> () {
    let amount = ic_cdk::api::call::msg_cycles_available128();
    if amount > 0 {
        ic_cdk::api::call::msg_cycles_accept128(amount);
    }
}
