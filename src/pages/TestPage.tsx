import { createSignal, onCleanup, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";

interface ProgressPayload {
    current: number;
    total: number;
    filename: string;
    status: string;
}

interface ConversionReport {
    total: number;
    successful: number;
    failed: number;
    skipped_non_image: number;
    failed_files: string[];
    skipped_files: string[];
    output_folder: string;
}

interface AnonymizationReport {
    total: number;
    successful: number;
    failed: number;
    failed_files: string[];
    output_folder: string;
}

export default function TestPage() {
    const [inputPath, setInputPath] = createSignal("");
    const [outputPath, setOutputPath] = createSignal("");
    const [progress, setProgress] = createSignal<ProgressPayload | null>(null);
    const [isProcessing, setIsProcessing] = createSignal(false);
    const [logs, setLogs] = createSignal<string[]>([]);

    // Options
    const [doAnonymize, setDoAnonymize] = createSignal(true);
    const [doConvert, setDoConvert] = createSignal(true);

    // Conversion Options
    const [skipExcel] = createSignal(false);

    // Anonymization Options
    const [tagsInput] = createSignal("0010,0010; 0010,0020; 0010,0030; 0008,0080; 0008,0090");
    const [replacementValue] = createSignal("ANONYMIZED");

    // Reports
    const [conversionReport, setConversionReport] = createSignal<ConversionReport | null>(null);
    const [anonymizationReport, setAnonymizationReport] = createSignal<AnonymizationReport | null>(null);

    const setupListeners = async () => {
        const unlistenConvert = await listen<ProgressPayload>("conversion_progress", (event) => {
            setProgress(event.payload);
            setLogs((prev) => [...prev, `[${event.payload.current}/${event.payload.total}] ${event.payload.status}: ${event.payload.filename}`]);
        });
        const unlistenAnonymize = await listen<ProgressPayload>("anonymization_progress", (event) => {
            setProgress(event.payload);
            setLogs((prev) => [...prev, `[${event.payload.current}/${event.payload.total}] ${event.payload.status}: ${event.payload.filename}`]);
        });

        onCleanup(() => {
            unlistenConvert();
            unlistenAnonymize();
        });
    };

    setupListeners();

    const handleSelectInput = async () => {
        const selected = await open({ directory: true, multiple: false });
        if (selected && typeof selected === "string") setInputPath(selected);
    };

    const handleSelectOutput = async () => {
        const selected = await open({ directory: true, multiple: false });
        if (selected && typeof selected === "string") setOutputPath(selected);
    };

    const resetState = () => {
        setLogs([]);
        setProgress(null);
        setConversionReport(null);
        setAnonymizationReport(null);
    };

    const handleProcess = async () => {
        if (!inputPath() || !outputPath()) return;
        if (!doAnonymize() && !doConvert()) {
            alert("Please select at least one operation.");
            return;
        }

        setIsProcessing(true);
        resetState();

        try {
            let currentInput = inputPath();
            let currentOutput = outputPath();
            let flattenConvert = false;

            // 1. Anonymize
            if (doAnonymize()) {
                // Parse tags
                const tags: [number, number][] = [];
                try {
                    const rawTags = tagsInput().split(";");
                    for (const raw of rawTags) {
                        const parts = raw.trim().split(",");
                        if (parts.length !== 2) throw new Error(`Invalid tag format: ${raw}`);
                        const group = parseInt(parts[0], 16);
                        const element = parseInt(parts[1], 16);
                        if (isNaN(group) || isNaN(element)) throw new Error(`Invalid hex values: ${raw}`);
                        tags.push([group, element]);
                    }
                } catch (e) {
                    alert(`Error parsing tags: ${e}`);
                    setIsProcessing(false);
                    return;
                }

                setLogs((prev) => [...prev, "Starting Anonymization..."]);
                const report = await invoke<AnonymizationReport>("anonymize_dicom", {
                    input: currentInput,
                    output: currentOutput,
                    tags: tags,
                    replacement: replacementValue(),
                });
                setAnonymizationReport(report);

                // Setup for next step (Conversion)
                // The anonymization creates [Input]_output/dicom_file
                // We want conversion to output to [Input]_output/png_file
                // So we set input for conversion to the anonymized dicom folder
                // And output for conversion to the root output folder ([Input]_output)
                // And tell conversion to flatten (not create another subfolder)
                currentInput = `${report.output_folder}/dicom_file`;
                currentOutput = report.output_folder;
                flattenConvert = true;
            }

            // 2. Convert
            if (doConvert()) {
                setLogs((prev) => [...prev, "Starting Conversion..."]);
                const report = await invoke<ConversionReport>("convert_dicom", {
                    input: currentInput,
                    output: currentOutput,
                    skipExcel: skipExcel(),
                    flattenOutput: flattenConvert,
                });
                setConversionReport(report);
            }

            setLogs((prev) => [...prev, "Processing completed successfully!"]);

        } catch (error) {
            console.error(error);
            setLogs((prev) => [...prev, `Error: ${error}`]);
        } finally {
            setIsProcessing(false);
        }
    };

    return (
        <div class="container mx-auto p-4 h-screen flex flex-col">
            <h1 class="text-3xl font-bold mb-6 text-primary">DICOM Processing Tool</h1>

            <div class="flex flex-col gap-4 mb-6">
                {/* Input/Output Selection */}
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div class="flex gap-2 items-center">
                        <input type="text" placeholder="Input Folder" value={inputPath()} readonly class="input input-bordered w-full" />
                        <button class="btn" onClick={handleSelectInput}>Select Input</button>
                    </div>
                    <div class="flex gap-2 items-center">
                        <input type="text" placeholder="Output Folder" value={outputPath()} readonly class="input input-bordered w-full" />
                        <button class="btn" onClick={handleSelectOutput}>Select Output</button>
                    </div>
                </div>

                {/* Operations Selection */}
                <div class="card bg-base-200 shadow-sm">
                    <div class="card-body p-4">
                        <h3 class="card-title text-lg">Operations</h3>
                        <div class="flex gap-6">
                            <label class="label cursor-pointer gap-2">
                                <input type="checkbox" checked={doAnonymize()} onChange={(e) => setDoAnonymize(e.currentTarget.checked)} class="checkbox checkbox-primary" />
                                <span class="label-text font-bold">Anonymize DICOM</span>
                            </label>
                            <label class="label cursor-pointer gap-2">
                                <input type="checkbox" checked={doConvert()} onChange={(e) => setDoConvert(e.currentTarget.checked)} class="checkbox checkbox-secondary" />
                                <span class="label-text font-bold">Convert to PNG</span>
                            </label>
                        </div>
                    </div>
                </div>

                {/* Configuration Options */}
                {/* <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <Show when={doAnonymize()}>
                        <div class="card bg-base-100 border border-base-300">
                            <div class="card-body p-4">
                                <h3 class="font-bold text-sm uppercase text-gray-500 mb-2">Anonymization Settings</h3>
                                <div class="form-control w-full">
                                    <label class="label"><span class="label-text">Tags (Group,Element; ...)</span></label>
                                    <input type="text" value={tagsInput()} onInput={(e) => setTagsInput(e.currentTarget.value)} class="input input-bordered input-sm w-full" />
                                </div>
                                <div class="form-control w-full">
                                    <label class="label"><span class="label-text">Replacement Value</span></label>
                                    <input type="text" value={replacementValue()} onInput={(e) => setReplacementValue(e.currentTarget.value)} class="input input-bordered input-sm w-full" />
                                </div>
                            </div>
                        </div>
                    </Show>

                    <Show when={doConvert()}>
                        <div class="card bg-base-100 border border-base-300">
                            <div class="card-body p-4">
                                <h3 class="font-bold text-sm uppercase text-gray-500 mb-2">Conversion Settings</h3>
                                <div class="form-control">
                                    <label class="label cursor-pointer justify-start gap-2">
                                        <input type="checkbox" checked={skipExcel()} onChange={(e) => setSkipExcel(e.currentTarget.checked)} class="checkbox checkbox-sm" />
                                        <span class="label-text">Skip Excel Report</span>
                                    </label>
                                </div>
                            </div>
                        </div>
                    </Show>
                </div> */}

                <button
                    class="btn btn-primary btn-lg w-full"
                    onClick={handleProcess}
                    disabled={isProcessing() || !inputPath() || !outputPath() || (!doAnonymize() && !doConvert())}
                >
                    {isProcessing() ? "Processing..." : "Start Processing"}
                </button>
            </div>

            {/* Progress Bar */}
            <Show when={progress()}>
                <div class="mb-6">
                    <div class="flex justify-between mb-1">
                        <span>Progress: {progress()?.current} / {progress()?.total}</span>
                        <span>{Math.round((progress()!.current / progress()!.total) * 100)}%</span>
                    </div>
                    <progress class="progress progress-primary w-full" value={progress()?.current} max={progress()?.total}></progress>
                    <p class="text-sm mt-1 text-gray-500 truncate">Processing: {progress()?.filename}</p>
                </div>
            </Show>

            {/* Results & Logs */}
            <div class="md:grid-cols-2 gap-4 flex-grow overflow-hidden">
                {/* <div class="flex flex-col h-full">
                    <h3 class="font-bold mb-2">Logs</h3>
                    <div class="bg-base-200 p-4 rounded-lg flex-grow overflow-y-auto font-mono text-xs">
                        <For each={logs()}>{(log) => <div>{log}</div>}</For>
                    </div>
                </div> */}

                <div class="flex flex-col h-full">
                    <h3 class="font-bold mb-2">Result Summary</h3>
                    <div class="bg-base-100 border border-base-300 p-4 rounded-lg flex-grow overflow-y-auto shadow-sm space-y-4">
                        <Show when={!anonymizationReport() && !conversionReport()}>
                            <div class="text-gray-400 italic">No results yet.</div>
                        </Show>

                        <Show when={anonymizationReport()}>
                            <div>
                                <h4 class="font-bold text-primary mb-2">Anonymization Results</h4>
                                <div class="stats stats-vertical lg:stats-horizontal shadow w-full">
                                    <div class="stat p-2">
                                        <div class="stat-title text-xs">Total</div>
                                        <div class="stat-value text-lg">{anonymizationReport()?.total}</div>
                                    </div>
                                    <div class="stat p-2">
                                        <div class="stat-title text-xs">Success</div>
                                        <div class="stat-value text-lg text-success">{anonymizationReport()?.successful}</div>
                                    </div>
                                    <div class="stat p-2">
                                        <div class="stat-title text-xs">Failed</div>
                                        <div class="stat-value text-lg text-error">{anonymizationReport()?.failed}</div>
                                    </div>
                                </div>
                                <Show when={anonymizationReport()?.failed_files.length! > 0}>
                                    <div class="text-error text-xs mt-2">
                                        Failed: {anonymizationReport()?.failed_files.join(", ")}
                                    </div>
                                </Show>
                            </div>
                        </Show>

                        <Show when={conversionReport()}>
                            <div>
                                <h4 class="font-bold text-secondary mb-2">Conversion Results</h4>
                                <div class="stats stats-vertical lg:stats-horizontal shadow w-full">
                                    <div class="stat p-2">
                                        <div class="stat-title text-xs">Total</div>
                                        <div class="stat-value text-lg">{conversionReport()?.total}</div>
                                    </div>
                                    <div class="stat p-2">
                                        <div class="stat-title text-xs">Success</div>
                                        <div class="stat-value text-lg text-success">{conversionReport()?.successful}</div>
                                    </div>
                                    <div class="stat p-2">
                                        <div class="stat-title text-xs">Skipped</div>
                                        <div class="stat-value text-lg text-warning">{conversionReport()?.skipped_non_image}</div>
                                    </div>
                                </div>
                                <Show when={conversionReport()?.failed_files.length! > 0}>
                                    <div class="text-error text-xs mt-2">
                                        Failed: {conversionReport()?.failed_files.join(", ")}
                                    </div>
                                </Show>
                            </div>
                        </Show>
                    </div>
                </div>
            </div>
        </div>
    );
}
