import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";
import { onMount, onCleanup } from "solid-js";
import { useNavigate } from "@solidjs/router";

import HomePage from "./pages/HomePage";
import LogsPage from "./pages/LogsPage";
import TagViewerPage from "./pages/TagViewerPage";

const App = (props: any) => {
    const navigate = useNavigate();

    const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === "F5") {
            e.preventDefault();
            navigate("/tags");
        }
    };

    onMount(() => {
        window.addEventListener("keydown", handleKeyDown);
    });

    onCleanup(() => {
        window.removeEventListener("keydown", handleKeyDown);
    });

    return props.children;
};

render(
    () => (
        <Router root={App}>
            <Route path="/" component={HomePage} />
            <Route path="/logs" component={LogsPage} />
            <Route path="/tags" component={TagViewerPage} />
        </Router>
    ),
    document.getElementById("root") as HTMLElement
);