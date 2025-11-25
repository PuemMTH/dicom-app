import { Component, createSignal, Show, createEffect, For, createMemo, onMount } from "solid-js";
import { makePersisted } from "@solid-primitives/storage";
import { A } from "@solidjs/router";
import { invoke } from "@tauri-apps/api/core";
import { createVirtualizer } from "@tanstack/solid-virtual";
import { open } from "@tauri-apps/plugin-dialog";
import StatsModal from "../components/StatsModal";

interface DicomTag {
    group: number;
    element: number;
    name: string;
    vr: string;
    value: string;
}

interface PinnedTag {
    group: number;
    element: number;
}

const TagViewerPage: Component = () => {
    const [tags, setTags] = createSignal<DicomTag[]>([]);
    // const [filteredTags, setFilteredTags] = createSignal<DicomTag[]>([]); // Replaced by memo
    const [filePath, setFilePath] = createSignal<string>("");
    const [folderPath, setFolderPath] = makePersisted(createSignal<string>(""), { name: "dicom-tag-viewer-path" });
    const [fileList, setFileList] = createSignal<string[]>([]);
    const [loading, setLoading] = createSignal(false);
    const [error, setError] = createSignal<string | null>(null);
    const [fileFilterText, setFileFilterText] = createSignal("");
    const [filterText, setFilterText] = createSignal("");
    const [pinnedTags, setPinnedTags] = createSignal<PinnedTag[]>([]);
    const [isDragging, setIsDragging] = createSignal(false);
    const [showStats, setShowStats] = createSignal(false);
    const [currentPage, setCurrentPage] = createSignal(1);
    const itemsPerPage = 20;

    let parentRef: HTMLDivElement | undefined;

    // Load pinned tags from local storage on mount
    const loadPinnedTags = () => {
        const stored = localStorage.getItem("pinnedTags");
        if (stored) {
            try {
                setPinnedTags(JSON.parse(stored));
            } catch (e) {
                console.error("Failed to parse pinned tags", e);
            }
        } else {
            setPinnedTags([
                { group: 0x0010, element: 0x0010 },
                { group: 0x0010, element: 0x0020 },
                { group: 0x0010, element: 0x0030 },
                { group: 0x0008, element: 0x0080 },
                { group: 0x0008, element: 0x0090 },
            ]);
        }
    };

    loadPinnedTags();

    // Save pinned tags to local storage whenever they change
    createEffect(() => {
        localStorage.setItem("pinnedTags", JSON.stringify(pinnedTags()));
    });

    const pinnedTagSet = createMemo(() => {
        const set = new Set<string>();
        pinnedTags().forEach(p => set.add(`${p.group}-${p.element}`));
        return set;
    });

    const isPinned = (tag: DicomTag) => {
        return pinnedTagSet().has(`${tag.group}-${tag.element}`);
    };

    const togglePin = (tag: DicomTag) => {
        if (isPinned(tag)) {
            setPinnedTags(prev => prev.filter(p => !(p.group === tag.group && p.element === tag.element)));
        } else {
            setPinnedTags(prev => [...prev, { group: tag.group, element: tag.element }]);
        }
    };

    // Filter and sort tags
    const filteredTags = createMemo(() => {
        let currentTags = tags();
        const filter = filterText().toLowerCase();

        if (filter) {
            currentTags = currentTags.filter(tag =>
                tag.name.toLowerCase().includes(filter) ||
                tag.value.toLowerCase().includes(filter) ||
                tag.group.toString(16).includes(filter) ||
                tag.element.toString(16).includes(filter)
            );
        }

        // Sort: Pinned first, then by group/element
        // Create a local reference to the set to avoid calling the signal in the loop if possible,
        // though calling the memo accessor is cheap.
        const pinnedSet = pinnedTagSet();

        // We need to copy the array before sorting to avoid mutating the original if it came from a store or similar,
        // though here it comes from tags() which is a signal of an array. 
        // Array.prototype.sort mutates in place.
        // filter() returns a new array, so we are safe if we filtered.
        // If we didn't filter, currentTags is tags(), which we shouldn't mutate.
        if (!filter) {
            currentTags = [...currentTags];
        }

        currentTags.sort((a, b) => {
            const aPinned = pinnedSet.has(`${a.group}-${a.element}`);
            const bPinned = pinnedSet.has(`${b.group}-${b.element}`);
            if (aPinned && !bPinned) return -1;
            if (!aPinned && bPinned) return 1;

            if (a.group !== b.group) return a.group - b.group;
            return a.element - b.element;
        });

        return currentTags;
    });

    const rowVirtualizer = createVirtualizer({
        get count() {
            return filteredTags().length;
        },
        getScrollElement: () => parentRef || null,
        estimateSize: () => 40,
        overscan: 5,
    });

    const handleDragOver = (e: DragEvent) => {
        e.preventDefault();
        setIsDragging(true);
    };

    const handleDragLeave = () => {
        setIsDragging(false);
    };

    const handleDrop = async (e: DragEvent) => {
        e.preventDefault();
        setIsDragging(false);

        let path = e.dataTransfer?.getData("text/plain");

        if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
            // @ts-ignore
            const droppedFile = e.dataTransfer.files[0];
            // @ts-ignore
            if (droppedFile.path) {
                // @ts-ignore
                path = droppedFile.path;
            }
        }

        if (path) {
            setFilePath(path);
            loadTags(path);
        }
    };

    const openFolder = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
            });
            if (selected && typeof selected === "string") {
                setFolderPath(selected);
                loadFileList(selected);
            }
        } catch (err) {
            console.error(err);
        }
    };

    const loadFileList = async (path: string) => {
        setLoading(true);
        try {
            const files = await invoke<string[]>("list_dicom_files", { folder: path });
            setFileList(files);
            setCurrentPage(1); // Reset to first page on new folder load
            // Removed auto-loading of the first file
            if (files.length > 0) {
                setFilePath(files[0]);
                loadTags(files[0]);
            }
        } catch (err) {
            setError(err as string);
        } finally {
            setLoading(false);
        }
    };

    const loadTags = async (path: string) => {
        if (!path) return;
        setLoading(true);
        setError(null);
        try {
            const result = await invoke<DicomTag[]>("get_dicom_tags", { path });
            setTags(result);
        } catch (err) {
            setError(err as string);
            setTags([]);
        } finally {
            setLoading(false);
        }
    };

    const filteredFileList = createMemo(() => {
        const filter = fileFilterText().toLowerCase();
        if (!filter) return fileList();
        return fileList().filter(file => file.toLowerCase().includes(filter));
    });

    // Reset page when filter changes
    createEffect(() => {
        fileFilterText();
        setCurrentPage(1);
    });

    onMount(() => {
        if (folderPath()) {
            loadFileList(folderPath());
        }
    });

    return (
        <div
            class={`h-screen flex flex-col bg-base-100 ${isDragging() ? "outline outline-4 outline-primary outline-offset-[-4px]" : ""}`}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
        >
            {/* Header */}
            <div class="flex flex-col border-b border-base-300 bg-base-100 shadow-sm z-20">
                <div class="navbar min-h-[3.5rem] px-4 gap-4">
                    <div class="flex-none">
                        <A href="/" class="btn btn-square btn-ghost btn-sm" title="Back to Home">
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5L3 12m0 0l7.5-7.5M3 12h18" />
                            </svg>
                        </A>
                    </div>
                    <div class="flex-1 items-center gap-2 overflow-hidden">
                        <h1 class="text-lg font-bold truncate text-base-content">DICOM Tag Viewer</h1>
                        <Show when={folderPath()}>
                            <span class="text-base-content/30 hidden sm:inline">/</span>
                            <span class="text-xs text-base-content/50 truncate font-mono hidden sm:inline" title={folderPath()}>{folderPath()}</span>
                        </Show>
                    </div>
                    <div class="flex-none gap-2">
                        <Show when={folderPath() && pinnedTags().length > 0}>
                            <button class="btn btn-sm btn-ghost text-primary" onClick={() => setShowStats(true)}>
                                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 mr-1">
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 3v11.25A2.25 2.25 0 006 16.5h2.25M3.75 3h-1.5m1.5 0h16.5m0 0h1.5m-1.5 0v11.25A2.25 2.25 0 0118 16.5h-2.25m-7.5 0h7.5m-7.5 0l-1 3m8.5-3l1 3m0 0l.5 1.5m-.5-1.5h-9.5m0 0l-.5 1.5m.75-9l3-3 2.148 2.148A12.061 12.061 0 0116.5 7.605" />
                                </svg>
                                Stats
                            </button>
                        </Show>
                        <button class="btn btn-sm btn-primary" onClick={openFolder}>
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 mr-1">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" />
                            </svg>
                            Open Folder
                        </button>
                    </div>
                </div>
                <Show when={loading()}>
                    <progress class="progress progress-primary w-full h-0.5 rounded-none absolute bottom-0 left-0"></progress>
                </Show>
            </div>

            <div class="flex flex-1 overflow-hidden">
                {/* Sidebar */}
                <Show when={fileList().length > 0}>
                    <div class="w-64 border-r border-base-300 bg-base-100 flex flex-col">
                        <div class="p-2 font-bold text-sm bg-base-200 border-b border-base-300 flex flex-col gap-2">
                            <div>Files ({filteredFileList().length})</div>
                            <input
                                type="text"
                                placeholder="Search files..."
                                class="input input-xs input-bordered w-full"
                                value={fileFilterText()}
                                onInput={(e) => setFileFilterText(e.currentTarget.value)}
                            />
                        </div>
                        <div class="flex-1 overflow-y-auto p-2">
                            <For each={filteredFileList().slice((currentPage() - 1) * itemsPerPage, currentPage() * itemsPerPage)}>
                                {(file) => (
                                    <div
                                        class={`p-2 text-xs cursor-pointer rounded hover:bg-base-200 truncate ${filePath() === file ? "bg-primary text-primary-content" : ""}`}
                                        onClick={() => {
                                            setFilePath(file);
                                            loadTags(file);
                                        }}
                                        title={file}
                                    >
                                        {file.split(/[/\\]/).pop()}
                                    </div>
                                )}
                            </For>
                        </div>
                        <div class="p-2 border-t border-base-300 flex justify-between items-center bg-base-200">
                            <button
                                class="btn btn-xs btn-ghost"
                                disabled={currentPage() === 1}
                                onClick={() => setCurrentPage(p => Math.max(1, p - 1))}
                            >
                                «
                            </button>
                            <span class="text-xs">
                                {currentPage()} / {Math.ceil(filteredFileList().length / itemsPerPage)}
                            </span>
                            <button
                                class="btn btn-xs btn-ghost"
                                disabled={currentPage() >= Math.ceil(filteredFileList().length / itemsPerPage)}
                                onClick={() => setCurrentPage(p => Math.min(Math.ceil(filteredFileList().length / itemsPerPage), p + 1))}
                            >
                                »
                            </button>
                        </div>
                    </div>
                </Show>

                {/* Main Content */}
                <div class="flex-1 flex flex-col overflow-hidden p-4 gap-4">
                    <Show when={filePath()}>
                        <div class="text-sm breadcrumbs">
                            <ul>
                                <li>{filePath()}</li>
                            </ul>
                        </div>
                    </Show>

                    <Show when={error()}>
                        <div class="alert alert-error">
                            <span>{error()}</span>
                        </div>
                    </Show>

                    <div class="flex-1 border border-base-300 rounded-lg overflow-hidden flex flex-col bg-base-100 shadow-sm">
                        <div class="p-2 bg-base-200 border-b border-base-300">
                            <input
                                type="text"
                                placeholder="Filter tags by name, value, or group/element..."
                                class="input input-sm input-bordered w-full"
                                value={filterText()}
                                onInput={(e) => setFilterText(e.currentTarget.value)}
                            />
                        </div>
                        <div class="grid grid-cols-12 gap-4 p-2 font-bold bg-base-200 border-b border-base-300 text-sm">
                            <div class="col-span-1 text-center">Pin</div>
                            <div class="col-span-2">Tag</div>
                            <div class="col-span-3">Name</div>
                            <div class="col-span-1">VR</div>
                            <div class="col-span-5">Value</div>
                        </div>

                        <div ref={parentRef} class="flex-1 overflow-auto p-2">
                            <div
                                style={{
                                    height: `${rowVirtualizer.getTotalSize()}px`,
                                    width: "100%",
                                    position: "relative",
                                }}
                            >
                                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                                    const tag = filteredTags()[virtualRow.index];
                                    const pinned = isPinned(tag);
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
                                            class={`grid grid-cols-12 gap-4 items-center hover:bg-base-200/50 px-2 border-b border-base-100 text-sm ${pinned ? "bg-yellow-50 dark:bg-yellow-900/20" : ""}`}
                                        >
                                            <div class="col-span-1 flex justify-center">
                                                <button
                                                    class={`btn btn-ghost btn-xs ${pinned ? "text-warning" : "text-base-content/30"}`}
                                                    onClick={() => togglePin(tag)}
                                                >
                                                    ★
                                                </button>
                                            </div>
                                            <div class="col-span-2 font-mono text-xs">
                                                ({tag.group.toString(16).padStart(4, "0").toUpperCase()},{tag.element.toString(16).padStart(4, "0").toUpperCase()})
                                            </div>
                                            <div class="col-span-3 truncate" title={tag.name}>{tag.name}</div>
                                            <div class="col-span-1">{tag.vr}</div>
                                            <div
                                                class="col-span-5 truncate font-mono text-xs"
                                                title={tag.value.length > 20 ? tag.value : tag.value.slice(0, 20) + "..."}
                                            >
                                                {/* {tag.value.length > 20 ? tag.value.slice(0, 20) + "..." : tag.value} */}
                                                {tag.value}
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>
                        </div>
                    </div>
                </div>
            </div >

            <Show when={showStats()}>
                <StatsModal
                    isOpen={showStats()}
                    onClose={() => setShowStats(false)}
                    folderPath={folderPath()}
                    pinnedTags={pinnedTags()}
                />
            </Show>
        </div >
    );
};

export default TagViewerPage;