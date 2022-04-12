import React from 'react'
import { quickstart_scaling_index } from "../../declarations/quickstart_scaling_index/index"
import {createActor} from './buckemoderatorent.js'
import {Principal} from '@dfinity/principal'

function AddModerator() {

    const [greeting, setGreeting] = React.useState([]);
    const [pending, setPending] = React.useState(false);
    const moderatorRef = React.useRef();

    const handleSubmit = async (e) => {
        e.preventDefault();
        // if (pending) return;
        setPending(true);
        
        const moderator = moderatorRef.current.value.toString();

        console.log(moderator, text);

        const moderatorPrincipal = Principal.fromText(moderator);

        const send_bucket = await quickstart_scaling_index.addContentModerator(moderatorPrincipal);

        console.log(zzz)

        setPending(false);
        return false;
    }


  return (
    <div className="box has-background-info">
        <h1>Add moderator (principal):</h1>
         <form onSubmit={handleSubmit}>
                <label htmlFor="moderator">moderator: &nbsp;</label>
                <input id="moderator" alt="moderator" type="text" ref={moderatorRef} />
                <button id="clickMeBtn" type="submit">Click Me!</button>
            </form>
    </div>
  )
}

export default AddModerator