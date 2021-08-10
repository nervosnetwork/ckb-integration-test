# ckb-bench

## Usage

* `$ ckb-bench dispatch --help`

```
dispatch capacity to users

USAGE:
    ckb-bench dispatch --capacity-per-cell <NUMBER> --cells-per-user <NUMBER> --data-dir <PATH> --n-users <NUMBER> --rpc-urls <URLS>...

OPTIONS:
        --capacity-per-cell <NUMBER>    Capacity per cell
        --cells-per-user <NUMBER>       Cells per user
        --data-dir <PATH>               Data directory [default: ./data]
        --n-users <NUMBER>              Number of users
        --rpc-urls <URLS>...            CKB rpc urls, prefix with network protocol, delimited by comma, e.g.
                                        "http://127.0.0.1:8114,http://127.0.0.2.8114"
```

* `$ ckb-bench bench --help`

```
bench the target ckb nodes

USAGE:
    ckb-bench bench [FLAGS] --bench-time-ms <TIME> --data-dir <PATH> --n-inout <NUMBER> --n-users <NUMBER> --rpc-urls <URLS>... --tx-interval-ms <TIME>

FLAGS:
        --is-smoking-test    Whether the target network is production network, like mainnet, testnet, devnet

OPTIONS:
        --bench-time-ms <TIME>     Bench time period
        --data-dir <PATH>          Data directory [default: ./data]
        --n-inout <NUMBER>         input-output pairs of a transaction
        --n-users <NUMBER>         Number of users
        --rpc-urls <URLS>...       CKB rpc urls, prefix with network protocol, delimited by comma, e.g.
                                   "http://127.0.0.1:8114,http://127.0.0.2.8114"
        --tx-interval-ms <TIME>    Interval of sending transactions in milliseconds
```

## Getting Started

### Bench on Testnet

1. Assume the owner privkey is `af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94`. Make sure this address has enough capacity.

2. Generate 9000 users, dispatch them 1 cells per user, 7100000000 capacity per cell.

  ```shell
  CKB_BENCH_OWNER_PRIVKEY=af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94 \
  ./ckb-bench dispatch \
    --data-dir data/ \
    --rpc-urls http://127.0.0.1:8111 \
    --capacity-per-cell 7100000000 \
    --cells-per-user 1 \
    --n-users 9000
  ```

3. Bench 300 seconds, benched transactions type are 2-in-2-out, sending transaction interval is 10ms

  ```shell
  CKB_BENCH_OWNER_PRIVKEY=af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94 \
  ./ckb-bench bench \
    --is-smoking-test \
    --data-dir data/ \
    --rpc-urls http://127.0.0.1:8111 \
    --bench-time-ms 300000 \
    --n-users 9000 \
    --n-inout 2 \
    --tx-interval-ms 10
  ```

2. Cleanup. Collect cells back to owner.

  ```shell
  CKB_BENCH_OWNER_PRIVKEY=af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94 \
  ./ckb-bench collect \
    --data-dir data/ \
    --rpc-urls http://127.0.0.1:8111 \
    --n-users 9000
  ```
