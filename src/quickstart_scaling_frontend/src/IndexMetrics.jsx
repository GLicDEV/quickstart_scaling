import React, {useState, useEffect} from 'react'


import { quickstart_scaling_index } from "../../declarations/quickstart_scaling_index"


const IndexMetrics = () => {

    const [metrics, setMetrics] = useState(0);

    useEffect(() => {
        const interval = setInterval(() => {
            const fetchMetrics = async () => {
                const data = await quickstart_scaling_index.getMetrics();
                setMetrics( data);
            }
            fetchMetrics();
        }, 1000);
        return () => clearInterval(interval);
      }, []);

  return (
    <section id="metrics">
        <br/ >
        <div className="content">
            <h1>Index Canister Metrics</h1>
            <div className="is-size-7"> <pre>{metrics}</pre>  </div>
        </div>
    </section>
  )
}

export default IndexMetrics