import { Component, createSignal, onMount, Show, For } from "solid-js";
import { A, useParams } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { makePersisted } from "@solid-primitives/storage";

interface TagValueDetail {
    value: string;
    count: number;
    files: string[];
}

interface TagDetails {
    group: number;
    element: number;
    name: string;
    values: TagValueDetail[];
}

const TagViewerDetailsPage: Component = () => {
    const params = useParams<{ group: string; element: string }>();
    const [folderPath] = makePersisted(createSignal<string>(""), { name: "dicom-tag-viewer-path" });
    const [details, setDetails] = createSignal<TagDetails | null>(null);
    const [loading, setLoading] = createSignal(false);
    const [error, setError] = createSignal<string | null>(null);
    const [progress, setProgress] = createSignal<{ current: number; total: number } | null>(null);
    const [expandedValue, setExpandedValue] = createSignal<string | null>(null);

    const fetchDetails = async () => {
        if (!folderPath() || !params.group || !params.element) return;

        setLoading(true);
        setError(null);
        setProgress(null);

        const group = parseInt(params.group);
        const element = parseInt(params.element);

        const unlisten = await import("@tauri-apps/api/event").then(mod =>
            mod.listen<{ current: number; total: number }>("tag_details_progress", (event) => {
                setProgress(event.payload);
            })
        );

        try {
            const result = await invoke<TagDetails>("get_tag_details", {
                folder: folderPath(),
                group,
                element,
            });
            setDetails(result);
        } catch (err) {
            setError(err as string);
        } finally {
            unlisten();
            setLoading(false);
            setProgress(null);
        }
    };

    onMount(() => {
        fetchDetails();
    });

    const toggleValue = (value: string) => {
        if (expandedValue() === value) {
            setExpandedValue(null);
        } else {
            setExpandedValue(value);
        }
    };

    return (
        <div class="h-screen flex flex-col bg-base-100">
            {/* Header */}
            <div class="flex flex-col border-b border-base-300 bg-base-100 shadow-sm z-20">
                <div class="navbar min-h-[3.5rem] px-4 gap-4">
                    <div class="flex-none">
                        <A href="/tags" class="btn btn-square btn-ghost btn-sm" title="Back to Tag Viewer">
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5L3 12m0 0l7.5-7.5M3 12h18" />
                            </svg>
                        </A>
                    </div>
                    <div class="flex-1 items-center gap-2 overflow-hidden">
                        <h1 class="text-lg font-bold truncate text-base-content">Tag Details</h1>
                        <Show when={details()}>
                            <span class="text-base-content/30 hidden sm:inline">/</span>
                            <span class="text-sm font-mono hidden sm:inline">
                                {details()?.name} ({details()?.group.toString(16).padStart(4, "0").toUpperCase()},{details()?.element.toString(16).padStart(4, "0").toUpperCase()})
                            </span>
                        </Show>
                    </div>
                </div>
                <Show when={loading()}>
                    <progress class="progress progress-primary w-full h-0.5 rounded-none absolute bottom-0 left-0"></progress>
                </Show>
            </div>

            <div class="flex-1 overflow-y-auto p-4">
                <Show when={loading()}>
                    <div class="flex flex-col justify-center items-center h-full space-y-4 opacity-50">
                        <span class="loading loading-spinner loading-lg"></span>
                        <Show when={progress()}>
                            <div class="flex flex-col items-center w-full max-w-md">
                                <div class="flex justify-between w-full text-sm mb-1">
                                    <span>Scanning files...</span>
                                    <span>{progress()?.current} / {progress()?.total}</span>
                                </div>
                                <progress
                                    class="progress progress-primary w-full"
                                    value={progress()?.current}
                                    max={progress()?.total}
                                ></progress>
                            </div>
                        </Show>
                    </div>
                </Show>

                <Show when={error()}>
                    <div class="alert alert-error">
                        <span>{error()}</span>
                    </div>
                </Show>

                <Show when={details() && !loading()}>
                    <div class="space-y-4">
                        <div class="stats shadow w-full bg-base-200">
                            <div class="stat">
                                <div class="stat-title">Total Unique Values</div>
                                <div class="stat-value">{details()?.values.length}</div>
                            </div>
                            <div class="stat">
                                <div class="stat-title">Most Common Value</div>
                                <div class="stat-value text-lg truncate" title={details()?.values[0]?.value}>
                                    {details()?.values[0]?.value || "N/A"}
                                </div>
                                <div class="stat-desc">Found in {details()?.values[0]?.count} files</div>
                            </div>
                        </div>

                        <div class="card bg-base-100 shadow-sm border border-base-300">
                            <div class="card-body p-0">
                                <table class="table table-zebra w-full">
                                    <thead>
                                        <tr>
                                            <th class="w-10"></th>
                                            <th>Value</th>
                                            <th class="w-32 text-right">Count</th>
                                            <th class="w-24 text-right">%</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For each={details()?.values}>
                                            {(item) => {
                                                const totalFiles = details()?.values.reduce((acc, v) => acc + v.count, 0) || 1;
                                                const percentage = ((item.count / totalFiles) * 100).toFixed(1);
                                                const isExpanded = () => expandedValue() === item.value;

                                                return (
                                                    <>
                                                        <tr class="hover cursor-pointer" onClick={() => toggleValue(item.value)}>
                                                            <td>
                                                                <svg
                                                                    xmlns="http://www.w3.org/2000/svg"
                                                                    fill="none"
                                                                    viewBox="0 0 24 24"
                                                                    stroke-width="1.5"
                                                                    stroke="currentColor"
                                                                    class={`w-4 h-4 transition-transform ${isExpanded() ? "rotate-90" : ""}`}
                                                                >
                                                                    <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5" />
                                                                </svg>
                                                            </td>
                                                            <td class="font-mono text-sm break-all">
                                                                {item.value || <span class="opacity-50 italic">&lt;empty&gt;</span>}
                                                            </td>
                                                            <td class="text-right font-mono">{item.count}</td>
                                                            <td class="text-right text-xs opacity-70">{percentage}%</td>
                                                        </tr>
                                                        <Show when={isExpanded()}>
                                                            <tr>
                                                                <td colspan={4} class="bg-base-200 p-0">
                                                                    <div class="p-4 max-h-64 overflow-y-auto">
                                                                        <h4 class="text-xs font-bold mb-2 opacity-70 uppercase tracking-wider">Files with this value ({item.files.length})</h4>
                                                                        <ul class="space-y-1">
                                                                            <For each={item.files}>
                                                                                {(file) => (
                                                                                    <li class="text-xs font-mono truncate hover:text-primary cursor-default" title={file}>
                                                                                        {file}
                                                                                    </li>
                                                                                )}
                                                                            </For>
                                                                        </ul>
                                                                    </div>
                                                                </td>
                                                            </tr>
                                                        </Show>
                                                    </>
                                                );
                                            }}
                                        </For>
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                </Show>
            </div>
        </div>
    );
};

export default TagViewerDetailsPage;