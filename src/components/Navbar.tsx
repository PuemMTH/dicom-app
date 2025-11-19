import { A } from "@solidjs/router";

const NavBar = (
) => {
    return (
        <header class="banner container mx-auto" role="banner">
            <nav class="navbar" role="navigation" aria-label="menu">
                <A class="btn btn-ghost text-xl" href="/">
                    Home
                </A>
                <A class="btn btn-ghost text-xl ml-4 log-button" href="/logs">
                    Logs
                </A>
            </nav>
        </header>
    );
};

export default NavBar;
