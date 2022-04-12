use crate::{businesslogic, CanisterEnv, Data, RuntimeState, RUNTIME_STATE};
use ic_cdk::print;
use ic_cdk_macros::{heartbeat, init, post_upgrade, pre_upgrade};

#[init]
fn init() {
    let env = Box::new(CanisterEnv::new());
    let data = Data::default();
    let runtime_state = RuntimeState { env, data };

    ic_cdk::print(format!("{}", ic_cdk::api::caller()));

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
    // re-index global_index tag2can
    RUNTIME_STATE.with(|state| businesslogic::reindex_tag_to_canisters(state.borrow_mut()));

    // check if we need to spawn new buckets and add any new planned buckets to the list
    if let true = RUNTIME_STATE.with(|state| businesslogic::should_spawn_buckets(state.borrow())) {
        // add spawn task
        print("plan to spawn a new bucket");
        RUNTIME_STATE.with(|state| state.borrow_mut().data.business_state.add_planned_bucket());
    }

    // spawn new buckets if needed. Note that the name "loop" here is a bit of a misnomer
    // as we aren't looping in the function that we are calling, but we can think of this
    // pattern as a loop that runs every heartbeat, and we get to visit it once per heartbeat
    businesslogic::spawn_bucket_loop().await;

    // TODO: add a module that checks for unfinished planned bucket installs and removes them

    businesslogic::push_moderators().await;
}
