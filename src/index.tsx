/* @refresh reload */
import { render } from 'solid-js/web'

import './index.css'
import App from './App'
import { createSignal } from 'solid-js'

const root = document.getElementById('root')
const [settingsOpen, setSettingsOpen] = createSignal(false)

window.addEventListener("keydown", function (event) {
  if (event.defaultPrevented) {
    return; // Do nothing if the event was already processed
  }

  switch (event.key) {
    case "s":
      console.log("s")
      setSettingsOpen(!settingsOpen())
      break;
    default:
      console.log(event.key)
      return; // Quit when this doesn't handle the key event.
  }

  // Cancel the default action to avoid it being handled twice
  event.preventDefault();
}, true);


render(() => <App settingsOpen={settingsOpen} />, root!)
