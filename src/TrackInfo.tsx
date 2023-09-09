
import { Accessor } from "solid-js"
import "./TrackInfo.css"
import { Track } from "./App"

export default (props: {
  track: Accessor<Track>
}) => {

  const { track} = props
  return <div id="track-info">
    {track().art.coverart ? <img src={track().art.coverart} /> : null}
    <div>
      <span id="track-name">{track().track}</span>
      <span id="artist-album">
        {track().artist ? <span>{track().artist}</span> : null} 
        {track().artist && track().album ? <span>・</span> : null}
        {track().album ? <span>{track().album}</span> : null} 
      </span>
    </div>
  </div>
  
}