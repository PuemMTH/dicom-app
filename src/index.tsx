import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";


import App from "./pages/App";
import Logs from "./pages/LogsPage";
import TestPage from "./pages/TestPage";


render(
    () => (
        <Router base="/">
            <Route path="/" component={App} />
            <Route path="/logs" component={Logs} />
            <Route path="/test" component={TestPage} />
        </Router>
    ),
    document.getElementById("root") as HTMLElement
);