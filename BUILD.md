
## Cross compiling using docker

Install `cross` to workaround a bug when building on remote docker.

```
cargo install cross --git "https://github.com/cross-rs/cross#f0ec688affed4"
```

Build for other architectures:

```
SET CROSS_REMOTE=1
make CARGO=cross
```

For now `windows-aarch64-msvc` is not supported by `rquickjs`.
