import { Accessor, Setter, createSignal } from 'solid-js'
import './App.css'
import Settings from './Settings'
import SignalRender from './SignalRender'
import { invoke } from '@tauri-apps/api/tauri'
import TrackInfo from './TrackInfo'

export type Track = {
  art : {
    background?: string
    coverart?: string
    coverarthq?: string
  }
  artist?: string
  track?: string
  album?: string
}

function App(props: {
  settingsOpen: Accessor<boolean>
  setSettingsOpen: Setter<boolean>
}) {

  const { settingsOpen, setSettingsOpen } = props
  const [track, setTrack] = createSignal<Track>({art: {
      coverart: "https://is5-ssl.mzstatic.com/image/thumb/Music112/v4/82/23/28/8223288b-0f80-54d4-c2ba-56a0d6a225f2/22UMGIM43681.rgb.jpg/400x400cc.jpg",
    },
    track: "Rock N Roll (feat. Kanye West & Kid Cudi)",
    artist: "Pusha T",
    album: "It's Almost Dry: Ye vs. Pharrell",
  })

  invoke("init_audio_capture", {})
  setInterval(async() => {
    invoke<Track>("recognize", {})
      .then((track) => {
        setTrack(track)
      })
      .catch((err: Error) => {
        console.error(err)
      });
    }, 10000);
  return (
    <div id='background' style={{
      "background-image": `url(${track().art.background})`,
    }}>
      {
        track() ? 
          <TrackInfo track={track} />
        : null
      }
      <SignalRender />
      {settingsOpen() ? <Settings exit={() => {console.log("exit"); setSettingsOpen(false)}} /> : null}
    </div>
  )
}

export default App
