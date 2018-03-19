use std::sync::{RwLock};
use std::time::{Instant, Duration};
use std::net::{SocketAddrV6};
use indexmap::IndexMap;
use indexmap::map::{Entry};
use rand::{self, Rng};

use utils::{check_addr};
use super::KEEPALIVE_CUTOFF;

#[derive(Clone, Copy, Debug)]
pub struct PeerInfo {
    last_seen: Instant
}

impl Default for PeerInfo {
    fn default() -> Self {
        PeerInfo {
            last_seen: Instant::now()
        }
    }
}

type Peers = IndexMap<SocketAddrV6, PeerInfo>;

#[derive(Debug)]
pub struct State {
    pub peers: RwLock<Peers>,
    pub inactive_peers: RwLock<Peers>,
}

impl State {
    pub fn new(initial_peers: Peers) -> Self {
        State {
            peers: RwLock::new(initial_peers),
            inactive_peers: RwLock::new(IndexMap::new()),
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.read().unwrap().len()
    }

    pub fn add_or_update_peer(&self, peer: SocketAddrV6, force: bool) -> bool {
        if !force {
            let inactive_map = self.inactive_peers.read().unwrap();
            if let Some(_) = inactive_map.get(&peer) {
                return false;
            }
        }
        let mut inactive_map = self.inactive_peers.write().unwrap();
        let mut map = self.peers.write().unwrap();
        if let Entry::Occupied(entry) = inactive_map.entry(peer) {
            map.insert(peer, *entry.get());
            entry.remove();
        }
        match map.entry(peer) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().last_seen = Instant::now();
                false
            },
            Entry::Vacant(entry) => {
                if check_addr(peer) {
                    entry.insert(PeerInfo::default());
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn prune_peers(&self) -> usize {
        let mut inactive_map = self.inactive_peers.write().unwrap();
        let mut map = self.peers.write().unwrap();
        let mut to_prune = Vec::new();
        for (addr, info) in map.iter() {
            if Instant::now() - info.last_seen > Duration::from_secs(KEEPALIVE_CUTOFF) {
                to_prune.push(*addr);
                inactive_map.insert(*addr, *info);
            }
        }
        for addr in to_prune.iter() {
            map.remove(addr);
        }
        to_prune.len()
    }
    
    pub fn remove_peer(&self, peer: SocketAddrV6) {
        let mut map = self.peers.write().unwrap();
        if let Entry::Occupied(entry) = map.entry(peer) {
            entry.remove();
        }
    }

    pub fn random_peers(&self, n: usize) -> Vec<SocketAddrV6> {
        let mut rng = rand::thread_rng();
        let peers = self.peers.read().unwrap();
        (0..n).into_iter().map(|_| {
            let idx = rng.gen_range::<usize>(0, peers.len());
            peers.get_index(idx).unwrap().0.clone()
        }).collect()
    }
}