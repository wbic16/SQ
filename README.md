Sququel
-------
SQ: The Simple Query Tool

SQ is a modern database, written from the ground-up to take advantage of Phext: 11-dimensional plain hypertext.

The name SQ was inspired by this tweet:
https://x.com/HSVSphere/status/1849817225038840016

# Getting Started
SQ leverages Rust and libphext as a core data store. All database primitives in SQ are built in terms of phext.

## Basic Commands

* sq <file>: launches a server that hosts a phext file via shared memory
* sq select <coord>: Fetches content from the current phext
* sq insert <coord> "text": Appends text at the specified coordinate
* sq update <coord> "text": Overwrites text at the specified coordinate
* sq delete <coord>: Removes all content from the specified coordinate
* sq save <file>: Writes the current phext back to disk
* sq init: Fast initialization for hosting world.phext from any state
* sq shutdown now: Instruct the daemon to terminate

# Developing

Shared memory handles are stored in the .sq directory where you invoked sq from.

## Linux
- reset.sh: removes `phext_link` and `phext_work` from the file system and starts an instance on `world.phext`

## Windows
- reset.ps1: same as reset.sh, but in PowerShell