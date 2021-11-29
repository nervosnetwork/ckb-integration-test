# CKB Integration Test

## Usage

### Run all cases

```shell
cargo run -- run --ckb2021 <path to ckb2021>
```

### Run specific cases

```shell
cargo run -- run --ckb2021 <path to ckb2021> --cases <cases name seperated by space>
```

### Run with setting loglevel

```shell
# loglevel=debug
cargo run -- run --ckb2021 <path to ckb2021> --debug

# loglevel=trace
cargo run -- run --ckb2021 <path to ckb2021> --debug
```
