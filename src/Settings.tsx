
import "./Settings.css"

import { invoke } from '@tauri-apps/api/tauri'
import { createSignal, onMount } from "solid-js"

export default (props: {
  exit: () => void
}) => {
  const { exit } = props

  const [devices, setDevices] = createSignal(["Default"])
  const [device, setDevice] = createSignal("Default")
  const [resMultiplier, setResMultiplier] = createSignal(1)

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

    invoke<number>("resolution", {})
      .then((resolution: number) => {
        setResMultiplier(Math.round(10 * resolution / 1024)/ 10) 
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
          setDevice(e.currentTarget.value)
        }}>
          {devices().map((op_device) => {
            return <option value={op_device} selected={op_device == device()}>{op_device}</option>
          })}
        </select>
      </div>
      <div>
        <h2>Resolution</h2>
        <span>
          <label for="resolution">{resMultiplier()}x</label>
          <input 
            type="range" name="resolution" id="resolution" 
            min={0} max={1} step={0.01}
            value={calcMultiplierInverse(resMultiplier(), 0.1, 10)} onInput={(e) => 
              setResMultiplier(calcResMultiplier(parseFloat(e.target.value), 0.1, 10))
            } />
        </span>
      </div>
      <div>
        <button onClick={async () => {
          let promises: Promise<any>[] = []        

          promises.push(invoke("change_device", { name: device() })
            .catch((err: Error) => {
              console.error(err)
            }));

          promises.push(invoke("set_resolution", { resolution: Math.round(resMultiplier() * 1024) })
            .catch((err: Error) => {
              console.error(err)
            }));

          
          await Promise.all(promises).then(async () => {    
              await invoke("stop_stream", {})
              return
            })
            .then(async() => {
              await invoke("init_audio_capture", {})
              return
            })
            .catch((err: Error) => {
              console.error(err)
            });
          exit()
        }}>
          Apply
        </button>
        <button onClick={exit}>
          Cancel
        </button>
      </div>
    </div>
  )
}

// 0 <= x <= 1
// https://www.desmos.com/calculator/bmpwql3xjc
const calcResMultiplier = (x: number, min: number, max: number) => {
  const a = 0.1
  const b = Math.log((max - min) / a + 1)
  const c = min - a

  return Math.round(10 * a * Math.exp(b * x) + c) /10
}
const calcMultiplierInverse = (y: number, min: number, max: number) => {
  const a = 0.1
  const b = Math.log((max - min) / a + 1)
  const c = min - a
  return Math.log((y - c) / a) / b
}