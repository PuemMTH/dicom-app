import { Component, For, Show } from "solid-js";

const ProgressLog: Component<{
  dicomProgress: { processed: number; total: number },
  pngProgress: { processed: number; total: number },
  logs: string[]
}> = (props) => {
  return (
    <div class="card bg-base-100 shadow-md mt-4 p-4">
      <h2 class="text-lg font-bold mb-2">Progress</h2>
      <div class="mb-2">
        <span>DICOM: {props.dicomProgress.processed}/{props.dicomProgress.total}</span>
        <progress class="progress progress-primary w-full" value={props.dicomProgress.processed} max={props.dicomProgress.total}></progress>
      </div>
      <div class="mb-2">
        <span>PNG: {props.pngProgress.processed}/{props.pngProgress.total}</span>
        <progress class="progress progress-secondary w-full" value={props.pngProgress.processed} max={props.pngProgress.total}></progress>
      </div>

      <Show when={props.logs.length > 0}>
        <h3 class="font-semibold mt-4">Log</h3>
        <div class="max-h-64 overflow-y-auto bg-gray-100 p-2 rounded">
          <For each={props.logs}>
            {(log) => <p class="text-sm">{log}</p>}
          </For>
        </div>
      </Show>
    </div>
  )
}

export default ProgressLog;