import * as React from 'react';
import { quickstart_scaling_index } from "../../declarations/quickstart_scaling_index"
import Bucket from './Bucket';
import ListBuckets from './ListBuckets';
import IndexMetrics from './IndexMetrics';
import PostContent from './PostContent';
import AddModerator from './AddModerator';
import FetchContent from './FetchContent';

const App = () => {
    const [greeting, setGreeting] = React.useState([]);
    const [pending, setPending] = React.useState(false);
    const inputRef = React.useRef();

    const handleSubmit = async (e) => {
        e.preventDefault();
        if (pending) return;
        setPending(true);
        const name = inputRef.current.value.toString();

        // Interact with hello actor, calling the greet method
        const greeting = await quickstart_scaling_index.getAllIndexes();
        
        console.log(greeting)

        setGreeting(greeting);
        setPending(false);
        return false;
    }

    return (
        <>
            
            <div className="columns is-multiline">
            
            <div className="column">
                <PostContent />
            </div>
            
            <div className="column">
                <AddModerator />
            </div>

            <div className="column">
                <FetchContent />
            </div>


            </div>

            <br />
            <IndexMetrics />
            <br />
            <ListBuckets />

        </>
    )
}

export default App;