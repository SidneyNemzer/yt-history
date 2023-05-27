# YouTube History

## Flamegraph on WSL 2

_Note: Does not work on WSL 1_

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
