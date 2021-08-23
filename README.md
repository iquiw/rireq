# rireq

[![CI](https://github.com/iquiw/rireq/workflows/Rust/badge.svg)](https://github.com/iquiw/rireq/actions)

Rireq is a simple bash history replacement.

## Requirements

### Runtime

* [Bash Preexec](https://github.com/rcaloras/bash-preexec)
* [fzf](https://github.com/junegunn/fzf)

## Setup

### Installation

```console
$ cargo install --git https://github.com/iquiw/rireq
```

### Configuration

Put the following in `~/.bashrc`.

```sh
eval "$(rireq init bash)"
```
