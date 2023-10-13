
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
        <span id="resolution">
          <label for="resolution">{Math.max(Math.round(10*resMultiplier()) / 10, 0.05)}x</label>
          <input 
            type="range" name="resolution"  
            min={0} max={1} step={0.000001}
            value={calcMultiplierInverse(resMultiplier())} onInput={(e) => 
              setResMultiplier(calcResMultiplier(parseFloat(e.target.value)))
            } />
        </span>
      </div>
      <div>
        <button onClick={async () => {
          let promises: Promise<any>[] = []        
          console.log("apply")

          await invoke("stop_stream", {})
          .catch((err: Error) => {
            console.error(err)
          });

          promises.push(invoke("change_device", { name: device() })
            .catch((err: Error) => {
              console.error(err)
            }));

          promises.push(invoke("set_resolution", { resolution: Math.round(resMultiplier() * 1024) })
            .catch((err: Error) => {
              console.error(err)
            }));

          
          await Promise.all(promises)
            .catch((err: Error) => {
              console.error(err)
            });
          console.log("starting stream capture")
          invoke("init_audio_capture", {})
            .catch((err: Error) => {
              console.error(err)
            });
          console.log("exiting callback")
          exit()

          console.log("done")
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
const calcResMultiplier = (x: number) => {
  const min = 0.01
  const max = 3
  const a = 0.970396
  const b = Math.log((max - min) / a + 1)
  const c = min - a

  return a * Math.exp(b * x) + c
}
const calcMultiplierInverse = (y: number) => {
  const min = 0.01
  const max = 3
  const a = 0.970396
  const b = Math.log((max - min) / a + 1)
  const c = min - a
  return Math.log((y - c) / a) / b
}