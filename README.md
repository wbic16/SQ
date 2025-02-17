Esque
-----
SQ: The Simple Query Tool

SQ is a modern database, written from the ground-up to take advantage of Phext: 11-dimensional plain hypertext. Like normal SQL databases, it has multiple pronunciations: "esque", "S-Q", and "Seek".


## Getting Started

you can either clone this repo and run `cargo build`, or just install the latest stable build: `cargo install sq`.

## Commands

SQ is designed to keep abstractions to a minimum. You can interact with phexts via shared memory (daemon "esque" mode) or a TCP socket ("seek" mode) with a simple REST API.

* sq help: displays online help
* sq <file>: launches a server that hosts a phext file via shared memory
* sq status: Displays daemon statistics (loaded phext, size, connection count)
* sq toc: Displays a textmap (list of available scrolls) of the currently-loaded phext
* sq push <coord> <file>: Overwrites the specified scroll with the local file
* sq pull <coord> <file>: Fetches the specified scroll to a local file
* sq select <coord>: Fetches content from the current phext
* sq insert <coord> "text": Appends text at the specified coordinate
* sq update <coord> "text": Overwrites text at the specified coordinate
* sq delete <coord>: Removes all content from the specified coordinate
* sq save <file>: Writes the current phext back to disk
* sq init: Fast initialization for hosting world.phext from any state
* sq shutdown: Instruct the daemon to terminate

# Modes of Operation

* `Daemon Mode`: If you supply a filename parameter to sq, it will launch in daemon mode - communicating with local system processes via shared memory
* `Listening Mode`: If you supply a port number to sq, it will launch in web server mode - listening on the TCP socket requested

## SQ Design Philosophy

SQ is a complete ground-up rewrite of database concepts. It probably doesn't have features you expect from a database. What it does offer is simplicity. SQ is designed to mirror computer architecture in 2025, not 1970. Databases are stored in phext files using variable-length scrolls. Essentially, everything in a phext database is a string.

SQ leverages Rust and libphext-rs to provide the core data store. All database primitives in SQ are built in terms of phext. For more information about the phext encoding format, refer to https://phext.io.

# Developing

In daemon mode, SQ uses shared memory to ensure that data transfers to/from the database engine are done as quickly as possible. The shared memory segments are managed by files stored in the .sq directory where you invoked SQ from. It is expected that you will run the client and the server from the same directory.

In listening mode, SQ reads and writes phexts via REST.

## REST API Endpoint

SQ offers a simple CRUD-style REST API. The API allows you to interact with multiple phexts from CURL or your web browser. Saving is automatic - if a command changes the content of a phext, it will be saved to disk immediately. Note that if you change the loaded phext without issuing a load command, SQ will automatically reload from disk first.

* /api/v2/load?p=<phext>: Loads the entire contents of `phext`.phext into the current context
* /api/v2/select?p=<phext>&c=<coordinate>: Fetches the scroll of text found at `coordinate` in `phext`.phext
* /api/v2/insert?p=<phext>&c=<coordinate>&s=<scroll>: Appends a scroll of text at `coordinate` in `phext`.phext
* /api/v2/update?p=<phext>&c=<coordinate>&s=<scroll>: Overwrites the contents of the scroll at `coordinate` in `phext`.phext
* /api/v2/delete?p=<phext>&c=<coordinate>: Clears the contents of the scroll at `coordinate` in `phext`.phext

## Linux
- `reset.sh`: removes the .sq folder from the file system and starts an instance on `world.phext`

## Windows
- `reset.ps1`: same as reset.sh, but in PowerShell

# Trivia

The name SQ was inspired by this tweet:
https://x.com/HSVSphere/status/1849817225038840016
