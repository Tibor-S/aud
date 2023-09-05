import { Accessor } from 'solid-js'
import './App.css'
import Settings from './Settings'
import SignalRender from './SignalRender'
import { invoke } from '@tauri-apps/api/tauri'

function App(props: {
  settingsOpen: Accessor<boolean>
}) {

  const { settingsOpen } = props

  invoke("init_audio_capture", {})
  return (
    <>
      <SignalRender />
      {settingsOpen() ? <Settings /> : null}
    </>
  )
}

export default App
