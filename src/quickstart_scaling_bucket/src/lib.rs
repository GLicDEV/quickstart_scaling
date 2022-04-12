mod businesslogic;
mod env;
mod lifetime;

use crate::env::{CanisterEnv, EmptyEnv, Environment, TimestampMillis};
use candid::{CandidType, Principal};
use ic_cdk::print;
use ic_cdk_macros::*;
use serde::Deserialize;

use crate::businesslogic::{BucketEntry, BucketIndex, BucketMetrics, EffectiveIndex};
use businesslogic::BusinessState;
use std::cell::{Ref, RefCell, RefMut};

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
struct BucketCanisterSettings {
    controllers: Vec<Principal>,
    index_canister_id: Option<Principal>,
    reindex_interval: TimestampMillis,
}

impl Default for BucketCanisterSettings {
    fn default() -> Self {
        BucketCanisterSettings {
            controllers: vec![],
            index_canister_id: None,
            // 5 seconds
            reindex_interval: 5_000_000_000,
        }
    }
}

#[derive(CandidType, Default, Deserialize)]
struct Data {
    canister_settings: BucketCanisterSettings,
    business_state: BusinessState,
    bucket_index: BucketIndex,
}

// MAIN FUNCTIONALITY

// Client facing functions are named using camelCase and are pretty self explanatory.
#[update(name = "postContent")]
fn post_content(tag: String, body: String) -> bool {
    RUNTIME_STATE.with(|state| post_content_impl(tag, body, &mut state.borrow_mut()))
}

fn post_content_impl(tag: String, body: String, runtime_state: &mut RefMut<RuntimeState>) -> bool {
    let entry = BucketEntry {
        tag,
        body,
        submitted_at: runtime_state.env.now(),
        submitted_by: runtime_state.env.caller(),
    };

    runtime_state.data.business_state.add_entry(entry)
}

// This gets all entries that were uploaded by the user or by an anonymous user.
#[query(name = "getByTag")]
fn get_by_tag(tag: String) -> Vec<BucketEntry> {
    RUNTIME_STATE.with(|state| get_by_tag_impl(tag, state.borrow()))
}

fn get_by_tag_impl(tag: String, runtime_state: Ref<RuntimeState>) -> Vec<BucketEntry> {
    let caller = runtime_state.env.caller();

    runtime_state.data.business_state.list_entries(&tag, caller)
}

// used for demoing the "moderator" ACL functionality
// A proper ACL implementation would be needed for production
#[query(name = "getAll", guard = "is_content_moderator")]
fn get_all() -> Vec<BucketEntry> {
    RUNTIME_STATE.with(|state| get_all_impl(state.borrow()))
}

fn get_all_impl(runtime_state: Ref<RuntimeState>) -> Vec<BucketEntry> {
    runtime_state.data.business_state.list_all_entries()
}

// Used for debug and demo purposes. Doesn't serve a business logic purpose.
// Could be changed to an "update" if the app needs to move to a pull index architecture
// (i.e. the Index canister would pull bucket index info). This would remove the
// need for heartbeat functionality on the bucket canister.
#[query(name = "getBucketIndex")]
fn get_bucket_index() -> EffectiveIndex {
    RUNTIME_STATE.with(|state| get_bucket_index_impl(state.borrow()))
}

fn get_bucket_index_impl(runtime_state: Ref<RuntimeState>) -> EffectiveIndex {
    // temp Debug
    // print(format!("Index: {:?}", runtime_state.data.bucket_index));

    runtime_state.data.bucket_index.effective_index.clone()
}

// The Index canister will update the moderators list using this update call
#[update(name = "add_content_moderators", guard = "is_controller")]
fn add_content_moderators(moderators: Vec<Principal>) {
    RUNTIME_STATE.with(|state| add_content_moderators_impl(moderators, state.borrow_mut()))
}

fn add_content_moderators_impl(
    moderators: Vec<Principal>,
    mut runtime_state: RefMut<RuntimeState>,
) {
    runtime_state
        .data
        .business_state
        .add_content_moderators(moderators)
}

// CANISTER LOGISTICS
// Metrics, cycles management and canister candid interface publishing
#[query(name = "getMetrics")]
fn get_metrics() -> BucketMetrics {
    RUNTIME_STATE.with(|state| get_metrics_impl(state.borrow()))
}

fn get_metrics_impl(runtime_state: Ref<RuntimeState>) -> BucketMetrics {
    BucketMetrics {
        canister_id: runtime_state.env.canister_id(),
        cycles_balance: runtime_state.env.cycles_balance(),
        controllers: runtime_state.data.canister_settings.controllers.clone(),
        moderators: runtime_state.data.business_state.get_content_moderators(),
        index_canister_id: runtime_state
            .data
            .canister_settings
            .index_canister_id
            .unwrap(),
        max_entries: runtime_state.data.business_state.max_entries(),
        current_entries: runtime_state.data.business_state.entries_count(),
        memory_used: runtime_state.env.memory_used(),
    }
}

// TODO: Use this to drain Buckets before removing them when cleaning up a deployment
//
#[update(name = "sendCycles", guard = "is_controller")]
async fn send_cycles() -> bool {
    let index_canister_id = RUNTIME_STATE.with(|state| {
        state
            .borrow()
            .data
            .canister_settings
            .index_canister_id
            .unwrap()
    });

    let cycles_amount = 50_000_000_000;

    send_cycles_impl(index_canister_id, cycles_amount).await
}

async fn send_cycles_impl(index_canister_id: Principal, cycles_amount: u128) -> bool {
    match ic_cdk::api::call::call_with_payment128(
        index_canister_id,
        "wallet_receive",
        {},
        cycles_amount,
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

// Accept cycles from the Index canister
#[update]
fn wallet_receive() -> () {
    let amount = ic_cdk::api::call::msg_cycles_available128();
    if amount > 0 {
        ic_cdk::api::call::msg_cycles_accept128(amount);
    }
}

// Announce our did interface
#[query(name = "__get_candid_interface_tmp_hack")]
fn __get_candid_interface_tmp_hack() -> String {
    let did = r#"
    
    type BucketEntry = record {
        tag: text;
        body: text; 
        submitted_at: nat64;
        submitted_by: principal;
    };
    
    type EffectiveIndex = record {
        tags: vec text;
        current_entries: nat64;
        bucket_max_entries: nat64;
    };

    type BucketMetrics = record {
        canister_id: principal;
        cycles_balance: nat;
        controllers: vec principal;
        moderators: vec principal;
        index_canister_id: principal;
        max_entries: nat64;
        current_entries: nat64;
        memory_used: nat64;
    };
    
    service : {
    "getMetrics" : () -> (BucketMetrics) query;
    "postContent" : (text, text) -> (bool);
    "getAll" : () -> (vec BucketEntry) query;
    "getByTag" : (text) -> (vec BucketEntry) query;
    "getBucketIndex" : () -> (EffectiveIndex) query;
    }
    "#;

    format!("{}", did)
}

// Guards:
fn is_controller() -> Result<(), String> {
    RUNTIME_STATE.with(|state| {
        if state
            .borrow()
            .data
            .canister_settings
            .controllers
            .contains(&state.borrow().env.caller())
        {
            Ok(())
        } else {
            Err("You are not a controller".to_string())
        }
    })
}

fn is_content_moderator() -> Result<(), String> {
    RUNTIME_STATE.with(|state| {
        if state
            .borrow()
            .data
            .business_state
            .get_content_moderators()
            .contains(&state.borrow().env.caller())
        {
            Ok(())
        } else {
            Err("You are not a moderator".to_string())
        }
    })
}
