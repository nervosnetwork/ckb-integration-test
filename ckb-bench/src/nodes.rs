use crate::util::wait_until;
use std::collections::hash_map::{Keys, Values};
use std::collections::HashMap;
use ckb_types::{ H256};
use std::collections::HashSet;
use ckb_jsonrpc_types::{BlockNumber,HeaderView};
use crate::node::Node;


pub struct Nodes {
    _inner: HashMap<String, Node>,
}

impl From<HashMap<String, Node>> for Nodes {
    fn from(nodes: HashMap<String, Node>) -> Self {
        Nodes { _inner: nodes }
    }
}

impl From<Vec<Node>> for Nodes {
    fn from(nodes: Vec<Node>) -> Self {
        nodes
            .into_iter()
            .map(|node| (node.node_name().to_string(), node))
            .collect::<HashMap<_, _>>()
            .into()
    }
}

impl From<Nodes> for HashMap<String, Node> {
    fn from(nodes: Nodes) -> Self {
        nodes._inner
    }
}

impl AsRef<HashMap<String, Node>> for Nodes {
    fn as_ref(&self) -> &HashMap<String, Node> {
        &self._inner
    }
}

impl Nodes {
    pub fn get_node(&self, node_name: &str) -> &Node {
        assert!(self._inner.contains_key(node_name));
        self._inner.get(node_name).expect("checked above")
    }

    pub fn get_node_mut(&mut self, node_name: &str) -> &mut Node {
        assert!(self._inner.contains_key(node_name));
        self._inner.get_mut(node_name).expect("checked above")
    }

    pub fn node_names(&self) -> Keys<String, Node> {
        self._inner.keys()
    }

    pub fn nodes(&self) -> Values<String, Node> {
        self._inner.values()
    }
}

impl Nodes {
    pub fn waiting_for_sync(&self) -> Result<(), Vec<(&str, BlockNumber, H256)>> {
        crate::trace!("Nodes::waiting_for_sync start");
        let highest_hashes: HashSet<H256> = {
            let tip_blocks: HashSet<_> = self.nodes().map(|node| node.get_tip_block()).collect();
            let tip_numbers = tip_blocks.iter().map(|block| block.header.inner.number);
            let highest_number = tip_numbers.max().unwrap();
            let highest_blocks = tip_blocks
                .into_iter()
                .filter(|block| block.header.inner.number == highest_number);
            highest_blocks.map(|block| block.header.hash).collect()
        };

        // 60 seconds is a reasonable timeout to sync, even for poor CI server
        let synced = wait_until(60, || {
            highest_hashes.iter().all(|hash| {
                self.nodes()
                    .all(|node| node.rpc_client().get_header(hash.clone()).unwrap().is_some())
            })
        });

        if !synced {
            let tips = self
                .nodes()
                .map(|node| {
                    let block = node.get_tip_block();
                    (node.node_name(), block.header.inner.number, block.header.hash)
                })
                .collect::<Vec<_>>();
            return Err(tips);
        }
        for node in self.nodes() {
            node.wait_for_tx_pool();
        }
        crate::trace!("Nodes::waiting_for_sync end");
        Ok(())
    }

    pub fn get_fixed_header(&self) -> HeaderView {
        let maximal_number = self
            .nodes()
            .map(|node| node.rpc_client().get_tip_block_number().unwrap())
            .min()
            .expect("at least 1 node");
        for number in (0..=maximal_number.into()).rev() {
            let headers = self
                .nodes()
                .map(|node| node.rpc_client().get_header_by_number(BlockNumber::from(number as u64) ).unwrap())
                .collect::<HashSet<_>>();
            if headers.len() == 1 {
                if let Some(first_value) = headers.iter().next() {
                    return first_value.clone().unwrap()
                }
            }
        }
        unreachable!()
    }
}

