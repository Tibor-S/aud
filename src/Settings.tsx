
import "./Settings.css"

import { invoke } from '@tauri-apps/api/tauri'
import { createSignal, onMount } from "solid-js"

export default () => {

  const [devices, setDevices] = createSignal(["Default"])
  const [device, setDevice] = createSignal("Default")

  onMount(() => {
    invoke<string[]>("query_devices", {})
      .then((devices: string[]) => {
        setDevices(devices)
      })
      .catch((err: Error) => {
        console.error(err)
      });

    invoke<string>("current_device", {})
      .then((device: string) => {
        setDevice(device)
      })
      .catch((err: Error) => {
        console.error(err)
      });
  })

  return (
    <div id="settings">
      <div>
        <h1>Settings</h1>
      </div>
      <div>
        <h2>Device</h2>
        <select id="device" onChange={async (e) => {
          await invoke<string>("change_device", { name: e.currentTarget.value })
            .then((new_device) => {
              setDevice(new_device)
              return
            })
            .catch((err: Error) => {
              console.error(err)
            });
            
          invoke("stop_stream", {})
            .then(() => {
              return invoke("init_audio_capture", {})
            })
            .catch((err: Error) => {
              console.error(err)
            });
        }}>
          {devices().map((op_device) => {
            return <option value={op_device} selected={op_device == device()}>{op_device}</option>
          })}
        </select>
      </div>
    </div>
  )
}