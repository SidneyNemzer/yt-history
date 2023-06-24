# yt-history/scripts

The scripts binary is a program for developing yt-history. In this directory run:

```
cargo run install-dev
```

This will build a release version of the scripts binary and copy it to the binaries location on your system.

By default, that's `/usr/local/bin` but you can change it by passing the destination folder: `cargo run install-dev /my/install/path`.

Now you can run:

```
ythdev
```

This is a shortcut to `cargo run --release` in the main binary project.

To uninstall the dev command, delete the file `ythdev` in your binaries directory (again, that's `/usr/local/bin` by default).
