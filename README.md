Esque
-----
SQ: The Simple Query Tool

SQ is a modern database, written from the ground-up to take advantage of Phext: 11-dimensional plain hypertext. Like normal SQL databases, it has two pronunciations: "esque" and "S-Q".

# Getting Started
SQ leverages Rust and libphext as a core data store. All database primitives in SQ are built in terms of phext.

## Basic Commands

* sq help: displays online help
* sq <file>: launches a server that hosts a phext file via shared memory
* sq select <coord>: Fetches content from the current phext
* sq insert <coord> "text": Appends text at the specified coordinate
* sq update <coord> "text": Overwrites text at the specified coordinate
* sq delete <coord>: Removes all content from the specified coordinate
* sq save <file>: Writes the current phext back to disk
* sq init: Fast initialization for hosting world.phext from any state
* sq shutdown now: Instruct the daemon to terminate

# Developing

SQ uses shared memory to ensure that data transfers to/from the database engine are done as quickly as possible. The shared memory segments are managed by files stored in the .sq directory where you invoke the server from. It is expected that you will run the client and the server from the same directory.

## Linux
- reset.sh: removes the .sq folder from the file system and starts an instance on `world.phext`

## Windows
- reset.ps1: same as reset.sh, but in PowerShell

# Trivia

The name SQ was inspired by this tweet:
https://x.com/HSVSphere/status/1849817225038840016