import './App.css'
import SignalRender from './SignalRender'
import { invoke } from '@tauri-apps/api/tauri'

function App() {
  invoke("init_audio_capture", {})
  return (
    <>
      <SignalRender />
    </>
  )
}

export default App
