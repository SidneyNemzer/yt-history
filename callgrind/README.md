# Callgrind

Callgrind (a tool included in Valgrind) is used to profile the call history of functions in a program. It's mainly used for C and C++ but also works with Rust.

## Callgrind on WSL2

Based on: http://www.codeofview.com/fix-rs/2017/01/24/how-to-optimize-rust-programs-on-linux/

_Callgrind will probably work on WSL 1, but viewing the profiles with kcachegrind will require the use of an X server on Windows, since its a GUI application._

Callgrind is included in some distros, including Ubuntu. Check with:

```sh
callgrind -h
```

Install kcachegrind:

```sh
sudo apt install kcachegrind
```

Start D-Bus:

```sh
./dbus-init.sh
```

Use callgrind:

```sh
cargo build --release
valgrind --tool=callgrind --dump-instr=yes --collect-jumps=yes --simulate-cache=yes target/release/yt-history
```

Open the file `callgrind.out.<PID>` in kcachegrind.
