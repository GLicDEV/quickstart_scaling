import React from 'react'
import { quickstart_scaling_index } from "../../declarations/quickstart_scaling_index/index"
import {createActor} from './bucketAgent.js'
import {Principal} from '@dfinity/principal'


function ContentItem(props) {
  return (
    <>
    
    <div>{props.record.tag}</div>
    <div>{props.record.body}</div>
    </>
  )
}


function FetchContent() {

    const [pending, setPending] = React.useState(false);
    const [contentList, setContentList] = React.useState([]);
    const [bucketCount, setBucketCount] = React.useState(0);
    const tagRef = React.useRef();

    const getContentFromBucket = async (tag, bucketId) => {
      const quickstart_scaling_bucket = createActor(bucketId);

      text = quickstart_scaling_bucket.getByTag(tag)

      return text;

    }

    const handleSubmit = async (e) => {
        e.preventDefault();
        // if (pending) return;
        setPending(true);
        setContentList([]);
        
        const tag = tagRef.current.value.toString();


        const bucket_list = await quickstart_scaling_index.getIndexByTag(tag);

        console.log(bucket_list)
        setBucketCount(bucket_list.length)

        setContentList([]);

        for (const bucket in bucket_list){
          console.log(bucket_list[bucket].toText())
          const content = await getContentFromBucket(tag,bucket_list[bucket].toText())
          console.log(content)

          const newcontent = contentList;
          setContentList([]);

          content.map(item => {
            
            console.log("Adding" + item.body)
            
            newcontent.push(item)
            console.log("Added. newcontent is now:" + newcontent.length)
  
            setContentList(newcontent);
          });
        

        }

        setPending(false);
        return false;
    }


  return (
    <>
    <div className="box has-background-info">
        <h1>Fetch content (by tag):</h1>
         <form onSubmit={handleSubmit}>
                <label htmlFor="tag">tag: &nbsp;</label>
                <input id="tag" alt="tag" type="text" ref={tagRef} />
                <button id="clickMeBtn" type="submit">Click Me!</button>
            </form>

          <div> Found this tag in {bucketCount} buckets </div>
          <div> {contentList.map(item => <ContentItem record={item} />)} </div>
    </div>
    </>
  )
}

export default FetchContent