# Usage

CKB-bench measures the CKB network's transaction throughput by sending many transactions. There are 4 steps:
  - Run a miner
  - Prepare enough unspent cells
  - Construct and send transactions continuously 
  - Generate on-chain report

### Run A Miner

In order to access enough CKB capacity, we have to mine blocks.
Assuming `CKB_BENCH_OWNER_PRIVKEY` corresponds to your configurated `block_assembler` of CKB `ckb.toml`.

```shell
  ckb-bench miner \
    --rpc-urls http://127.0.0.1:8111 \
    --mining-interval-ms 1000
```

The command runs a thread that mines blocks every `1000` milliseconds.

- `--mining-interval-ms 1000`: Delay 1000 milliseconds between mining continuous blocks

### Prepare Enough Unspent Cells

We will need a lot of unspent cells to be used as transaction inputs in order to construct plenty of transactions.
Assuming `CKB_BENCH_OWNER_PRIVKEY` has so much CKB capacity, the following code derives `9000` users from `CKB_BENCH_OWNER_PRIVKEY`, transfers `1` cells per user, and gives each cell `7100000000` capacity.

```shell
  CKB_BENCH_OWNER_PRIVKEY=af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94 \
  ckb-bench dispatch \
    --rpc-urls http://127.0.0.1:8111 \
    --n-users 9000 \
    --cells-per-user 1 \
    --capacity-per-cell 7100000000
```

- `--n-users 9000`: Generate `9000` derived users
- `--cells-per-user 1`: Dispatch `1` unspent cell to every derived user.
- `--capacity-per-cell 7100000000`: Gives each cell `7100000000` capacity.

### Construct and Send Transactions Continuously

CKB-bench provides several options for specifying benchmark scenarios. Here is an example:

  ```shell
  CKB_BENCH_OWNER_PRIVKEY=af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94 \
  ./ckb-bench bench \
    --rpc-urls http://127.0.0.1:8111 \
    --n-users 9000 \
    --n-inout 2 \
    --bench-time-ms 300000 \
    --tx-interval-ms 10 \
    --concurrent-requests 10 
  ```

- `--n-users 9000`: Use the `9000` derived users to bench
- `--n-inout 2`: Construct 2-in-2-out transactions
- `--bench-time-ms 300000`: Bench `300000` milliseconds
- `--tx-interval-ms 10`: Delay 10 milliseconds between sending continuous transactions
- `--concurrent-requests 10` : 10 users are conducting load testing simultaneously.

Ckb-bench continuously performs these tasks for `bench-time-ms` duration:
  - collects unspent cells of derived users
  - and constructs specified transactions from them
  - and sends transactions with a delay of *tx-interval-ms* between sending continuous transactions

### Generate On-chain Report

After benching, CKB-bench generates an on-chain report. Also, you can do it via `ckb-bench stat`.

Here is an example of an on-chain report:

| ckb_version | transactions_per_second | n_inout | n_nodes | delay_time_ms | average_block_time_ms | average_block_transactions | average_block_transactions_size | from_block_number | to_block_number | total_transactions | total_transactions_size | transactions_size_per_second |
| :---------- | :------------- | :------ | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 0.102.0 | 97 | 1 | 1 | 0 | 3013 | 292 | 119243 | 1377 | 1426 | 14642 | 5962165 | 39571 |
| 0.102.0 | 108 | 2 | 1 | 0 | 1233 | 133 | 82941 | 1634 | 1755 | 16289 | 10118818 | 67231 |

If you are interested in the measurement approach, I recommend reading the source code. [On-chain report explaining](https://github.com/nervosnetwork/ckb-integration-test/blob/d57011f8d140d5f4dc56dc147d7babe2a1cec322/ckb-bench/src/stat.rs#L6-L39):

```rust
/// On-chain report
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Report {
    /// Number of running CKB nodes
    pub n_nodes: usize,
    /// Number of transaction inputs and outputs
    pub n_inout: usize,
    /// Client version of the running CKB nodes
    pub ckb_version: String,
    /// Delay time between sending continuous transactions, equal to `--tx-interval-ms`
    pub delay_time_ms: Option<u64>,

    /// The chain height when starting benchmark
    pub from_block_number: BlockNumber,
    /// The chain height when ending benchmark
    pub to_block_number: BlockNumber,

    /// On-chain transactions per seconds
    pub transactions_per_second: u64,
    /// On-chain transaction size per seconds
    pub transactions_size_per_second: u64,

    /// Average block transactions
    pub average_block_transactions: usize,
    /// Average block transactions size
    pub average_block_transactions_size: usize,
    /// Average block interval in milliseconds
    pub average_block_time_ms: u64,

    /// Total transactions
    pub total_transactions: usize,
    /// Total transactions size
    pub total_transactions_size: usize,
}
```
