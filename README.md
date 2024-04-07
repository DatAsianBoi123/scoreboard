# Scoreboard
[Link Coming Soon...](about:blank)

A scoreboard website designed to be used with FIRST robotics competitions.

## Features
- Hosting game sessions
- Creating custom games (not implemented yet)
- Viewing games
- Reffing games

## Locally Hosting

The website is hosted online, which means no local hosting is required!
However, if you do need to host locally, you can follow the following steps:

1. Install rust on [their website](https://rust-lang.org/tools/install)
2. Install `cargo-binstall` by running `cargo install cargo-binstall` (this may take a while!)
<br>
Alternatively, you can visit [their github](https://github.com/cargo-bins/cargo-binstall?tab=readme-ov-file#installation) to install pre-built binaries
3. Install `cargo-shuttle` by running `cargo binstall shuttle`
4. `clone` this repository and navigate to its root folder
5. Run `cargo shuttle run` and wait for the project to compile and run
6. Finally, navigate to [127.0.0.1:8000](http://127.0.0.1:8000) on your favorite web browser and begin hosting games!

**Note:** Using this method will only allow your machine to view the website and connect to the server.
To allow anyone on your local network to join, run `cargo shuttle run` with the `--external` flag and open port 8000 on your firewall.
To do this on Linux, you can follow the steps [here](https://digitalocean.com/community/tutorials/opening-a-port-on-linux).
If you're running WSL, you can view [this SA answer and thread](https://stackoverflow.com/a/66890232).
