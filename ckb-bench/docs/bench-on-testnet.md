# Bench on Testnet

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

3. Bench 300 seconds, benched transactions type are 2-in-2-out, sending transaction interval is 10ms ,10 users

  ```shell
  CKB_BENCH_OWNER_PRIVKEY=af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94 \
  ./ckb-bench bench \
    --is-smoking-test \
    --data-dir data/ \
    --rpc-urls http://127.0.0.1:8111 \
    --bench-time-ms 300000 \
    --n-users 9000 \
    --n-inout 2 \
    --tx-interval-ms 10 \ 
    --concurrent-requests 10 
  ```

2. Cleanup. Collect cells back to owner.

  ```shell
  CKB_BENCH_OWNER_PRIVKEY=af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94 \
  ./ckb-bench collect \
    --data-dir data/ \
    --rpc-urls http://127.0.0.1:8111 \
    --n-users 9000
  ```
