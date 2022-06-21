# Naivechain implementation in Rust

## A short note

This was meant to be an implementation of https://github.com/lhartikk/naivechain (a lot of concepts and ideas are borrowed from there) in
Rust but the code of this project turned out to be more complex.
I didn't want to use websockets and decided to use TcpStreams.
Because of bad design decisions I made this program opens a new tcp connection every time
a message is sent which makes tracking peers not really easy.
Because of this limitation an unneeded greeting mechanism between peers was needed.

Nevertheless this project was a good learning experience for me. This repo might also
serve as a starting point for others who want to see a basic blockchain implementation
Rust.

## Getting started

Set up three client connected to each other.

Run these commands in different terminal emulators or in a terminal multiplexer.

```sh
HTTP_PORT="4000" P2P_PORT="5000" cargo run
```

```sh
INITIAL="localhost:5000" HTTP_PORT="4001" P2P_PORT="5001" cargo run
```

```sh
INITIAL="localhost:5000" HTTP_PORT="4002" P2P_PORT="5002" cargo run
```

You can see the peers of each node with this command: `curl localhost:{HTTP_PORT of desired node}/peers`

To see the block a node thinks exist run the command: `curl localhost:{HTTP_PORT of desired node}/blocks`

To mine a block: `curl --data '{content of the new block}' localhost:{HTTP_PORT of desired node}/mineBlock`
