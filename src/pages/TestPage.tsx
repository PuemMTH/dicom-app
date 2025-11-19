import { createSignal, onCleanup, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";

interface ProgressPayload {
    current: number;
    total: number;
    filename: string;
    status: string;
}

export default function TestPage() {
    const [inputPath, setInputPath] = createSignal("");
    const [outputPath, setOutputPath] = createSignal("");
    const [skipExcel, setSkipExcel] = createSignal(false);
    const [progress, setProgress] = createSignal<ProgressPayload | null>(null);
    const [isConverting, setIsConverting] = createSignal(false);
    const [logs, setLogs] = createSignal<string[]>([]);

    const setupListener = async () => {
        const unlisten = await listen<ProgressPayload>("conversion_progress", (event) => {
            setProgress(event.payload);
            setLogs((prev) => [...prev, `[${event.payload.current}/${event.payload.total}] ${event.payload.status}: ${event.payload.filename}`]);
        });
        onCleanup(() => unlisten());
    };

    setupListener();

    const handleSelectInput = async () => {
        const selected = await open({
            directory: true,
            multiple: false,
        });
        if (selected && typeof selected === "string") {
            setInputPath(selected);
        }
    };

    const handleSelectOutput = async () => {
        const selected = await open({
            directory: true,
            multiple: false,
        });
        if (selected && typeof selected === "string") {
            setOutputPath(selected);
        }
    };

    const handleConvert = async () => {
        if (!inputPath() || !outputPath()) return;

        setIsConverting(true);
        setLogs([]);
        setProgress(null);

        try {
            await invoke("convert_dicom", {
                input: inputPath(),
                output: outputPath(),
                skipExcel: skipExcel(),
            });
            setLogs((prev) => [...prev, "Conversion completed!"]);
        } catch (error) {
            console.error(error);
            setLogs((prev) => [...prev, `Error: ${error}`]);
        } finally {
            setIsConverting(false);
        }
    };

    return (
        <div class="container mx-auto p-4">
            <h1 class="text-2xl font-bold mb-4">DICOM Conversion Test</h1>

            <div class="flex flex-col gap-4 mb-6">
                <div class="flex gap-2 items-center">
                    <input
                        type="text"
                        placeholder="Input Folder"
                        value={inputPath()}
                        readonly
                        class="input input-bordered w-full"
                    />
                    <button class="btn" onClick={handleSelectInput}>Select Input</button>
                </div>

                <div class="flex gap-2 items-center">
                    <input
                        type="text"
                        placeholder="Output Folder"
                        value={outputPath()}
                        readonly
                        class="input input-bordered w-full"
                    />
                    <button class="btn" onClick={handleSelectOutput}>Select Output</button>
                </div>

                <div class="form-control">
                    <label class="label cursor-pointer justify-start gap-2">
                        <input
                            type="checkbox"
                            checked={skipExcel()}
                            onChange={(e) => setSkipExcel(e.currentTarget.checked)}
                            class="checkbox"
                        />
                        <span class="label-text">Skip Excel Report</span>
                    </label>
                </div>

                <button
                    class="btn btn-primary"
                    onClick={handleConvert}
                    disabled={isConverting() || !inputPath() || !outputPath()}
                >
                    {isConverting() ? "Converting..." : "Start Conversion"}
                </button>
            </div>

            {progress() && (
                <div class="mb-6">
                    <div class="flex justify-between mb-1">
                        <span>Progress: {progress()?.current} / {progress()?.total}</span>
                        <span>{Math.round((progress()!.current / progress()!.total) * 100)}%</span>
                    </div>
                    <progress
                        class="progress progress-primary w-full"
                        value={progress()?.current}
                        max={progress()?.total}
                    ></progress>
                    <p class="text-sm mt-1 text-gray-500">Processing: {progress()?.filename}</p>
                </div>
            )}

            <div class="bg-base-200 p-4 rounded-lg h-64 overflow-y-auto font-mono text-sm">
                <For each={logs()}>{(log) => <div>{log}</div>}</For>
            </div>
        </div>
    );
}
