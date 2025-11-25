import { Component } from "solid-js";
import { A, useParams } from "@solidjs/router";

const TagViewerDetailsPage: Component = () => {
    const { group, element } = useParams<{ group: string; element: string }>();
    return (
        <div>
            <h1>Tag Viewer Details Page</h1>
            <p>Group: {group}</p>
            <p>Element: {element}</p>
            <A href="/tags">Back to Tag Viewer</A>
        </div>
    );
};

export default TagViewerDetailsPage;