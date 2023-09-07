import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api'
import { createSignal, onCleanup, onMount } from "solid-js";
import { JSX } from "solid-js/jsx-runtime";
import "./SignalRender.css"

export default (props: JSX.CanvasHTMLAttributes<HTMLCanvasElement>) => {
  
  const [audioSignal, setAudioSignal] = createSignal<number[]>([])
  const [ctxHeight, setCtxHeight] = createSignal(300)
  const [ctxWidth, setCtxWidth] = createSignal(400)

  
  let canvas: HTMLCanvasElement = document.createElement('canvas');
  onMount(() => {
    const ctx = canvas.getContext("2d");
    setCtxHeight(canvas.clientHeight)
    setCtxWidth(canvas.clientWidth)
    window.addEventListener('resize', () => {
      console.log("resize", canvas.clientWidth, canvas.clientHeight)
      setCtxHeight(canvas.clientHeight)
      setCtxWidth(canvas.clientWidth)
    })
    let frame = requestAnimationFrame(loop);

    function loop() {
      frame = requestAnimationFrame(loop);
      if (ctx === null) return;
      const signal = audioSignal()
      if (signal.length < 2) return;
      const height = ctxHeight()
      const width = ctxWidth()
      const dx = width / (signal.length - 1)

      ctx.clearRect(0, 0, width, height);
      ctx.strokeStyle = "black";
      ctx.beginPath();
      ctx.moveTo(0, (signal[0] + 1) * height / 2);
      for (let i = 0; i < signal.length; i++) {
        const x = i * dx
        const y = (signal[i] + 1) * height / 2
        ctx.lineTo(x, y)
      }
      ctx.stroke()
    }

    onCleanup(() => cancelAnimationFrame(frame));
  });
  
  listen<number[]>('signal', (event) => {
    // console.log("window_event (get-data):", payload)
    setAudioSignal(event.payload)
  })

  setInterval(() => {
    invoke("emit_signal", {});
  }, 1000 / 60)

  return <canvas id="signal-render" ref={canvas} width={ctxWidth()} height={ctxHeight()} {...props}/>
}