# syncterm
Final project for the EECS 396: Systems Programming in Rust course at Northwestern University.

A library for managing a network of synchronized command-line applications, such as networked CLI games.

See the `examples/` directory for example projects!

## Git Helper example
To run the git_helper example, run:
```
$ cargo run --example=git_helper
```
in one terminal to start up the server, and:
```
$ cargo run --example=git_helper <username>
```
in another terminal to start up a client.
