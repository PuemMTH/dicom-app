import { Component, createSignal, Show, For, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

interface TagStat {
    group: number;
    element: number;
    name: string;
    value_counts: Record<string, number>;
}

interface StatsModalProps {
    isOpen: boolean;
    onClose: () => void;
    folderPath: string;
    pinnedTags: { group: number; element: number }[];
}

const StatsModal: Component<StatsModalProps> = (props) => {
    const [stats, setStats] = createSignal<TagStat[]>([]);
    const [loading, setLoading] = createSignal(false);
    const [error, setError] = createSignal<string | null>(null);

    // Fetch stats when the modal opens
    const fetchStats = async () => {
        if (!props.folderPath || props.pinnedTags.length === 0) return;

        setLoading(true);
        setError(null);
        try {
            const result = await invoke<TagStat[]>("get_pinned_tags_stats", {
                folder: props.folderPath,
                tags: props.pinnedTags.map(t => [t.group, t.element]),
            });
            setStats(result);
        } catch (err) {
            setError(err as string);
        } finally {
            setLoading(false);
        }
    };

    // Watch for open state changes
    // In SolidJS, we can just call fetchStats when isOpen becomes true if we wrap it in an effect,
    // or just call it from the parent. But let's use an effect here.
    // Actually, simpler: call fetchStats when the component mounts if open, or when props change.
    // Since the modal might be conditionally rendered, onMount is good.
    onMount(() => {
        fetchStats();
    });

    return (
        <dialog class={`modal ${props.isOpen ? "modal-open" : ""}`}>
            <div class="modal-box w-11/12 max-w-5xl h-[80vh] flex flex-col">
                <div class="flex justify-between items-center mb-4">
                    <h3 class="font-bold text-lg">Tag Statistics</h3>
                    <button class="btn btn-sm btn-circle btn-ghost" onClick={props.onClose}>✕</button>
                </div>

                <div class="flex-1 overflow-y-auto">
                    <Show when={loading()}>
                        <div class="flex justify-center items-center h-full">
                            <span class="loading loading-spinner loading-lg"></span>
                        </div>
                    </Show>

                    <Show when={error()}>
                        <div class="alert alert-error">
                            <span>{error()}</span>
                        </div>
                    </Show>

                    <Show when={!loading() && !error()}>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <For each={stats()}>
                                {(stat) => (
                                    <div class="card bg-base-200 shadow-sm">
                                        <div class="card-body p-4">
                                            <h4 class="card-title text-sm">
                                                {stat.name}
                                                <span class="text-xs font-mono opacity-50">
                                                    ({stat.group.toString(16).padStart(4, "0").toUpperCase()},{stat.element.toString(16).padStart(4, "0").toUpperCase()})
                                                </span>
                                            </h4>

                                            <div class="mt-2 space-y-2">
                                                {(() => {
                                                    const total = Object.values(stat.value_counts).reduce((a, b) => a + b, 0);
                                                    const sortedEntries = Object.entries(stat.value_counts)
                                                        .sort(([, a], [, b]) => b - a)
                                                        .slice(0, 10); // Top 10 values

                                                    return (
                                                        <>
                                                            <For each={sortedEntries}>
                                                                {([value, count]) => {
                                                                    const percentage = ((count / total) * 100).toFixed(1);
                                                                    return (
                                                                        <div class="text-xs">
                                                                            <div class="flex justify-between mb-1">
                                                                                <span class="truncate font-mono" title={value}>{value || "<empty>"}</span>
                                                                                <span class="opacity-70">{count} ({percentage}%)</span>
                                                                            </div>
                                                                            <progress
                                                                                class="progress progress-primary w-full"
                                                                                value={count}
                                                                                max={total}
                                                                            ></progress>
                                                                        </div>
                                                                    );
                                                                }}
                                                            </For>
                                                            <Show when={Object.keys(stat.value_counts).length > 10}>
                                                                <div class="text-xs text-center opacity-50 mt-1">
                                                                    ...and {Object.keys(stat.value_counts).length - 10} more values
                                                                </div>
                                                            </Show>
                                                        </>
                                                    );
                                                })()}
                                            </div>
                                        </div>
                                    </div>
                                )}
                            </For>
                        </div>
                        <Show when={stats().length === 0}>
                            <div class="text-center opacity-50 mt-10">
                                No pinned tags to display statistics for.
                            </div>
                        </Show>
                    </Show>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop">
                <button onClick={props.onClose}>close</button>
            </form>
        </dialog>
    );
};

export default StatsModal;
