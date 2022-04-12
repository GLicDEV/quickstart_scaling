import { Actor, HttpAgent } from "@dfinity/agent";


  export const idlFactory = ({ IDL }) => {
    const BucketEntry = IDL.Record({
      'tag' : IDL.Text,
      'body' : IDL.Text,
      'submitted_at' : IDL.Nat64,
      'submitted_by' : IDL.Principal,
    });
    const EffectiveIndex = IDL.Record({
      'tags' : IDL.Vec(IDL.Text),
      'bucket_max_entries' : IDL.Nat64,
      'current_entries' : IDL.Nat64,
    });
    const BucketMetrics = IDL.Record({
      'cycles_balance' : IDL.Nat,
      'controllers' : IDL.Vec(IDL.Principal),
      'memory_used' : IDL.Nat64,
      'canister_id' : IDL.Principal,
      'max_entries' : IDL.Nat64,
      'current_entries' : IDL.Nat64,
      'index_canister_id' : IDL.Principal,
      'moderators' : IDL.Vec(IDL.Principal),
    });
    return IDL.Service({
      'getAll' : IDL.Func([], [IDL.Vec(BucketEntry)], ['query']),
      'getBucketIndex' : IDL.Func([], [EffectiveIndex], ['query']),
      'getByTag' : IDL.Func([IDL.Text], [IDL.Vec(BucketEntry)], ['query']),
      'getMetrics' : IDL.Func([], [BucketMetrics], ['query']),
      'postContent' : IDL.Func([IDL.Text, IDL.Text], [IDL.Bool], []),
    });
  };

export const createActor = (canisterId, options) => {
    const agent = new HttpAgent({ ...options?.agentOptions });
    
    // Fetch root key for certificate validation during development
    if(process.env.NODE_ENV !== "production") {
      agent.fetchRootKey().catch(err=>{
        console.warn("Unable to fetch root key. Check to ensure that your local replica is running");
        console.error(err);
      });
    }
  
    // Creates an actor with using the candid interface and the HttpAgent
    return Actor.createActor(idlFactory, {
      agent,
      canisterId,
      ...options?.actorOptions,
    });
  };