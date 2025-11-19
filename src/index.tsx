import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";


import App from "./pages/App";
import Logs from "./pages/LogsPage";


render(
    () => (
        <Router base="/">
            <Route path="/" component={App} />
            <Route path="/logs" component={Logs} />
        </Router>
    ),
    document.getElementById("root") as HTMLElement
);