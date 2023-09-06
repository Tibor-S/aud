import { Accessor, Setter } from 'solid-js'
import './App.css'
import Settings from './Settings'
import SignalRender from './SignalRender'
import { invoke } from '@tauri-apps/api/tauri'

function App(props: {
  settingsOpen: Accessor<boolean>
  setSettingsOpen: Setter<boolean>
}) {

  const { settingsOpen, setSettingsOpen } = props

  invoke("init_audio_capture", {})
  return (
    <>
      <SignalRender />
      {settingsOpen() ? <Settings exit={() => setSettingsOpen(false)} /> : null}
    </>
  )
}

export default App
