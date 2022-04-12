import React from 'react'
import { quickstart_scaling_index } from "../../declarations/quickstart_scaling_index"
import {createActor} from './bucketAgent.js'


function PostContent() {

    const [greeting, setGreeting] = React.useState("");
    const [pending, setPending] = React.useState(false);
    const tagRef = React.useRef();
    const textRef = React.useRef();

    const handleSubmit = async (e) => {
        e.preventDefault();
        // if (pending) return;
        setPending(true);
        
        const tag = tagRef.current.value.toString();
        const text = textRef.current.value.toString();

        console.log(tag, text);

        const send_bucket = await quickstart_scaling_index.getUploadOrder();

        console.log("Should send to " + send_bucket[0].toText())

        const quickstart_scaling_bucket = createActor(send_bucket[0].toText());

        const response = await quickstart_scaling_bucket.postContent(tag,text);

        console.log(response)

        if (response){
            setGreeting("Sent " + tag + " " + text + " to " + send_bucket[0].toText())
        }

        setPending(false);
        return false;
    }


  return (
    <div className="box has-background-info">
        <h1>Post content:</h1>
         <form onSubmit={handleSubmit}>
                <label htmlFor="tag">Tag: &nbsp;</label>
                <input id="tag" alt="tag" type="text" ref={tagRef} />
                <label htmlFor="text">Text: &nbsp;</label>
                <input id="text" alt="text" type="text" ref={textRef} />
                <button id="clickMeBtn" type="submit">Click Me!</button>
            </form>
        {greeting}
    </div>
  )
}

export default PostContent