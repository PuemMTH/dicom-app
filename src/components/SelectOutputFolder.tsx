import { Component } from "solid-js";

const SelectOutputFolder: Component<{ path: string; onSelect: () => void }> = (props) => {
    return (
        <div class="flex items-center gap-3">
            <button
                onClick={props.onSelect}
                class="w-30 px-3 py-1.5 text-sm rounded bg-blue-500 text-white hover:bg-blue-600"
            >
                Output Folder
            </button>
            <span class="text-sm">{`📁 ${props.path || "No folder selected"}`}</span>
        </div>
    );
};

export default SelectOutputFolder;
