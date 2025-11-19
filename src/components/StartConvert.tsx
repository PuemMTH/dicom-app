import { Component } from "solid-js";

const StartConvert: Component<{ onStart: () => void }> = (props) => {
    return (
        <div class="flex justify-end">
            <button
                onClick={props.onStart}
                class="px-4 py-2 text-sm rounded bg-blue-500 text-white hover:bg-blue-600"
            >
                Start Convert
            </button>
        </div>
    );
};

export default StartConvert;
