import React,{useState, useEffect} from 'react'

import {createActor} from './bucketAgent.js'

// 'cycles_balance' : IDL.Nat,
// 'controllers' : IDL.Vec(IDL.Principal),
// 'memory_used' : IDL.Nat64,
// 'canister_id' : IDL.Principal,
// 'max_entries' : IDL.Nat64,
// 'current_entries' : IDL.Nat64,
// 'index_canister_id' : IDL.Principal,
// 'moderators' : IDL.Vec(IDL.Principal),

const Bucket = (props) => {

    const quickstart_scaling_bucket = createActor(props.canisterId);

    const [metrics, setMetrics] = useState({});

    useEffect(() => {
        const interval = setInterval(() => {
            const fetchMetrics = async () => {
                const data = await quickstart_scaling_bucket.getMetrics();
                setMetrics( data);
                // console.log(data)
            }
            fetchMetrics();
        }, 1000);
        return () => clearInterval(interval);
      }, []);

  return (
    <div className="column ">
       <div className="box has-background-success"> 
       
        {/* {Object.entries(metrics).map(([key, val]) => 
                    <h2 key={key}>{key}: {val.toString()}</h2>)
                  } */}

       {
       metrics.hasOwnProperty('memory_used') &&
       <>
       <div className="has-text-link"> {metrics.canister_id.toString()} </div>
       <div className="has-text-danger is-size-4 has-text-weight-bold"> Entries: {metrics.current_entries.toString()} / {metrics.max_entries.toString()} </div>
       <div className=""> Moderators: {metrics.moderators.toString()} </div>
       
       </>
      }
        

       </div>

       
    </div>
  )
}

export default Bucket