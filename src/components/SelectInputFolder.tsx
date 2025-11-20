import { Component, createSignal } from "solid-js";

const SelectInputFolder: Component<{ path: string; onSelect: () => void; onPathChange: (path: string) => void }> = (props) => {
    const [isDragOver, setIsDragOver] = createSignal(false);

    const handleDragOver = (e: DragEvent) => {
        e.preventDefault();
        setIsDragOver(true);
    };

    const handleDragLeave = () => {
        setIsDragOver(false);
    };

    const handleDrop = (e: DragEvent) => {
        e.preventDefault();
        setIsDragOver(false);
        if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
            // @ts-ignore - Tauri provides the path property on File objects
            const path = e.dataTransfer.files[0].path;
            if (path) {
                props.onPathChange(path);
            } else {
                // Fallback for web or if path is missing (might need specific handling)
                // @ts-ignore
                const name = e.dataTransfer.files[0].name;
                // In some tauri setups, name might be the full path or we might need to handle it differently.
                // But usually 'path' property is present in Tauri.
                if (name) props.onPathChange(name);
            }
        }
    };

    return (
        <div
            class={`flex items-center gap-3 p-2 rounded border-2 border-dashed transition-colors ${isDragOver() ? "border-blue-500 bg-blue-50" : "border-transparent"}`}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
        >
            <button
                onClick={props.onSelect}
                class="w-30 px-3 py-1.5 text-sm rounded bg-blue-500 text-white hover:bg-blue-600"
            >
                Input Folder
            </button>
            <span class="text-sm">{`📁 ${props.path || "No folder selected"}`}</span>
        </div>
    );
};

export default SelectInputFolder;
