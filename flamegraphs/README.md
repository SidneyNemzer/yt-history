# Flamegraphs

1. `01-7f3632.svg` -- Parsed in 6.10 seconds. Generated with `cargo flamegraph --root -F 10000`.
2. `02-a7d968.svg` -- Parsed in 3.39 seconds. Generated with `cargo flamegraph -F 10000`.
3. `03-4b667b.svg` -- Parsed in 835.6 ms. Generated with `cargo flamegraph -F 10000`.

Files in this directory document performance changes over time. They're created with Flamegraph. See below for setup instructions.

Flamegraph files are named in the format:

```
<number>-<commit>.svg
```

Where `<number>` keeps flamegraphs in order by time created and `<commit>` is the Git commit that the Flamegraph was created from.

## Flamegraph on WSL 2

_Does not work on WSL 1. Some of these steps can be skipped if you're on Ubuntu, non-WSL._

Install dependencies. perf can be built without some of these but it will be missing critical functionality like function names.

```sh
sudo apt update
sudo apt install flex bison
sudo apt install libdwarf-dev libelf-dev libnuma-dev libunwind-dev \
  libnewt-dev libdwarf++0 libelf++0 libdw-dev libbfb0-dev \
  systemtap-sdt-dev libssl-dev libperl-dev python-dev-is-python3 \
  binutils-dev libiberty-dev libzstd-dev libcap-dev libbabeltrace-dev
```

Build perf:

```sh
git clone https://github.com/microsoft/WSL2-Linux-Kernel --depth 1
cd WSL2-Linux-Kernel/tools/perf
make
sudo cp perf /usr/local/bin
```

Use Flamegraph:

```sh
cargo install flamegraph
cargo flamegraph --root -F 10000 # take more samples than the default ~1000
```
