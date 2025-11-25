import { Component, Show } from "solid-js";
import { createVirtualizer } from "@tanstack/solid-virtual";

export interface LogEntry {
    file_name: string;
    file_path: string;
    success: boolean;
    status: string;
    message: string;
    conversion_type: string;
}

interface LogViewerProps {
    logs: LogEntry[];
    isOpen: boolean;
    onClose: () => void;
}

const LogViewer: Component<LogViewerProps> = (props) => {
    let parentRef: HTMLDivElement | undefined;

    const rowVirtualizer = createVirtualizer({
        get count() {
            return props.logs.length;
        },
        getScrollElement: () => parentRef || null,
        estimateSize: () => 50, // Estimate row height
        overscan: 5,
    });

    const getStatusColor = (status: string) => {
        switch (status?.toLowerCase()) {
            case "success": return "badge-success";
            case "failed": return "badge-error";
            case "skipped": return "badge-warning";
            default: return "badge-ghost";
        }
    };

    return (
        <div class={`modal ${props.isOpen ? "modal-open" : ""}`}>
            <div class="modal-box w-11/12 max-w-6xl h-[80vh] flex flex-col p-0 bg-base-100">
                <div class="flex items-center justify-between p-4 border-b border-base-300 bg-base-200 sticky top-0 z-10">
                    <h3 class="font-bold text-lg">Process Logs ({props.logs.length})</h3>
                    <button onClick={props.onClose} class="btn btn-sm btn-circle btn-ghost">✕</button>
                </div>

                {/* Header Row */}
                <div class="grid grid-cols-12 gap-4 p-4 font-bold bg-base-200 border-b border-base-300 text-sm uppercase">
                    <div class="col-span-2">Status</div>
                    <div class="col-span-1">Type</div>
                    <div class="col-span-3">File Name</div>
                    <div class="col-span-3">Path</div>
                    <div class="col-span-3">Message</div>
                </div>

                {/* Virtualized List */}
                <div
                    ref={parentRef}
                    class="flex-1 overflow-auto p-4"
                >
                    <div
                        style={{
                            height: `${rowVirtualizer.getTotalSize()}px`,
                            width: "100%",
                            position: "relative",
                        }}
                    >
                        <Show when={props.logs.length > 0} fallback={
                            <div class="absolute inset-0 flex items-center justify-center text-base-content/50">
                                No logs available yet.
                            </div>
                        }>
                            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                                const log = props.logs[virtualRow.index];
                                return (
                                    <div
                                        style={{
                                            position: "absolute",
                                            top: 0,
                                            left: 0,
                                            width: "100%",
                                            height: `${virtualRow.size}px`,
                                            transform: `translateY(${virtualRow.start}px)`,
                                        }}
                                        class="grid grid-cols-12 gap-4 items-center hover:bg-base-200/50 px-2 border-b border-base-100 text-sm"
                                    >
                                        <div class="col-span-2">
                                            <div class={`badge ${getStatusColor(log.status)} gap-2 text-white badge-sm`}>
                                                {log.status == "Skipped" ? "Already exists" : log.success ? "Success" : "Failed"}
                                            </div>
                                        </div>
                                        <div class="col-span-1 truncate" title={log.conversion_type}>{log.conversion_type}</div>
                                        <div class="col-span-3 font-medium truncate" title={log.file_name}>{log.file_name}</div>
                                        <div class="col-span-3 text-base-content/70 truncate" title={log.file_path}>
                                            {log.file_path}
                                        </div>
                                        <div class="col-span-3 text-base-content/70 truncate" title={log.message}>
                                            {log.message}
                                        </div>
                                    </div>
                                );
                            })}
                        </Show>
                    </div>
                </div>

                <div class="p-4 border-t border-base-300 bg-base-200 flex justify-end">
                    <button onClick={props.onClose} class="btn">Close</button>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop">
                <button onClick={props.onClose}>close</button>
            </form>
        </div>
    );
};

export default LogViewer;
