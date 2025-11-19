import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";

import HomePage from "./pages/HomePage";
import LogsPage from "./pages/LogsPage";

render(
    () => (
        <Router base="/">
            <Route path="/" component={HomePage} />
            <Route path="/logs" component={LogsPage} />
        </Router>
    ),
    document.getElementById("root") as HTMLElement
);