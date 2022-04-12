use crate::businesslogic::IndexState;
use crate::{BucketIndex, CanisterEnv, Data, RuntimeState, RUNTIME_STATE};
use candid::Deserialize;
use ic_cdk::api::call::CallResult;
use ic_cdk::export::candid::CandidType;
use ic_cdk::export::Principal;
use ic_cdk::print;
use ic_cdk_macros::{heartbeat, init, post_upgrade, pre_upgrade};
use std::cell::RefMut;

#[init]
fn init() {
    let env = Box::new(CanisterEnv::new());
    let data = Data::default();
    let mut runtime_state = RuntimeState { env, data };

    let caller_id = ic_cdk::api::caller();

    ic_cdk::print(format!("Bucket spawned by {}", caller_id));

    runtime_state
        .data
        .canister_settings
        .controllers
        .push(caller_id);
    runtime_state.data.canister_settings.index_canister_id = Some(caller_id);

    #[derive(CandidType, Deserialize, Debug, Default)]
    struct SendArgs {
        greet: String,
        controllers: Vec<Principal>,
    }

    let call_arg = ic_cdk::api::call::arg_data::<(Option<SendArgs>,)>().0;

    ic_cdk::print(format!("{:?}", call_arg));

    // Add the additional controllers received from the Index canister
    for controller in call_arg.unwrap_or(SendArgs::default()).controllers.iter() {
        runtime_state
            .data
            .canister_settings
            .controllers
            .push(controller.clone());
    }

    RUNTIME_STATE.with(|state| *state.borrow_mut() = runtime_state);
}

#[pre_upgrade]
fn pre_upgrade() {
    RUNTIME_STATE.with(|state| ic_cdk::storage::stable_save((&state.borrow().data,)).unwrap());
}

#[post_upgrade]
fn post_upgrade() {
    let env = Box::new(CanisterEnv::new());
    let (data,): (Data,) = ic_cdk::storage::stable_restore().unwrap();
    let runtime_state = RuntimeState { env, data };

    RUNTIME_STATE.with(|state| *state.borrow_mut() = runtime_state);
}

#[heartbeat]
async fn heartbeat() {
    // re-index
    RUNTIME_STATE.with(|state| generate_bucket_index(&mut state.borrow_mut()));

    // send index

    // Send index only if it's in the New state (this prevents multiple attempts
    // at sending the same index if one attempt lasts longer and another
    // heartbeat is triggered.
    if let IndexState::New =
        RUNTIME_STATE.with(|state| state.borrow().data.bucket_index.index_state)
    {
        send_index().await
    }
}

async fn send_index() {
    // Take ownership of the task
    let rand_id = RUNTIME_STATE.with(|state| {
        let some_rand = state.borrow_mut().env.random_u32();
        state.borrow_mut().data.bucket_index.index_state = IndexState::InSync(some_rand);
        some_rand
    });

    let effective_index =
        RUNTIME_STATE.with(|state| state.borrow().data.bucket_index.effective_index.clone());

    let index_canister_id = RUNTIME_STATE
        .with(|state| state.borrow().data.canister_settings.index_canister_id)
        .unwrap();

    // Actually send the index
    let call_succeeded: CallResult<(bool,)> =
        ic_cdk::api::call::call(index_canister_id, "add_bucket_index", (effective_index,)).await;

    if let Err((code, msg)) = call_succeeded {
        print(format!("Error! Code:{:?} Msg:{:?}", code, msg));
        // Set the task to new so the next iteration of heartbeat can work on it
        RUNTIME_STATE
            .with(|state| state.borrow_mut().data.bucket_index.index_state = IndexState::New);
    } else {
        // Set the task to completed
        // Check if it is still the task we took ownership of before await.
        if let IndexState::InSync(lock) =
            RUNTIME_STATE.with(|state| state.borrow().data.bucket_index.index_state)
        {
            if lock == rand_id {
                // Actually set the task to completed
                RUNTIME_STATE.with(|state| {
                    state.borrow_mut().data.bucket_index.index_state = IndexState::Synced
                });
            }
            // Else? What should we do if the state got rolled back while awaiting?
            // Set the task to new and pretend nothing happened?
            // Let the task stuck in InSync and add a module that deals with stuck tasks?
            else {
                RUNTIME_STATE.with(|state| {
                    state.borrow_mut().data.bucket_index.index_state = IndexState::New
                });
            }
        }
    }
}

fn generate_bucket_index(runtime_state: &mut RefMut<RuntimeState>) {
    //Only re-index if the last index is older than canister_settings.reindex_interval
    if runtime_state.env.now() - runtime_state.data.bucket_index.last_updated
        > runtime_state.data.canister_settings.reindex_interval
    {
        ic_cdk::api::print(format!(
            "re-index {} {} {} {}",
            runtime_state.env.now(),
            runtime_state.data.bucket_index.last_updated,
            runtime_state.data.canister_settings.reindex_interval,
            runtime_state.env.now() - runtime_state.data.bucket_index.last_updated
        ));
        let effective_index = runtime_state.data.business_state.create_bucket_index();

        runtime_state.data.bucket_index = BucketIndex {
            effective_index,
            index_state: IndexState::New,
            last_updated: runtime_state.env.now(),
        };
    }
}
