# ckb sync pending tx

### prepare setup
- init job dir and
- prepare `big-tx` data
```shell
sh script/sync-pending-tx.sh setup
```

### mod ssh service and nodes msg 
- job/benchmark-*/inventory.yml
- job/ansible/vars/node*yml


###  run sync test 
- deploy node1
- deploy node2
- deploy node3
- deploy node4
- wait node4 tx-pool tx(8000) load   
- link p2p node1-node2
- link p2p node1-node3
- link p2p node2-node4
- link p2p node3-node4
- start node1 miner 
```shell
sh script/sync-pending-tx.sh run
```
### clean env
- clean node1 data
- clean node2 data
- clean node3 data
- clean node4 data
```shell
sh script/sync-pending-tx.sh clean_ckb_env
```

### todo
- [ ] metric 
- [ ] terraform