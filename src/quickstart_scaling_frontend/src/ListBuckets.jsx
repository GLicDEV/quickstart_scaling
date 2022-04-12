import React,{useState, useEffect} from 'react'

import { quickstart_scaling_index } from "../../declarations/quickstart_scaling_index"

import Bucket from './Bucket'


const ListBuckets = () => {

    const [buckets, setBuckets] = useState([]);

    useEffect(() => {
        const interval = setInterval(() => {
            const fetchMetrics = async () => {
                const data = await quickstart_scaling_index.getAllIndexes();
                setBuckets(data);
            }
            fetchMetrics();
        }, 1000);
        return () => clearInterval(interval);
      }, []);

  return (
      <>
      <br/ >
      <div className="content">
            <h1>Buckets Metrics</h1>
        </div>
    <div className="columns is-multiline">
        
        { buckets.map(bucket => <Bucket key={bucket.toString()} canisterId={bucket.toString()} />)} 
        
    </div>
      </>
  )
}

export default ListBuckets