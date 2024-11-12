// src/node.rs
use std::collections::{HashSet, HashMap};
use tokio::sync::mpsc;
use libp2p::{
    PeerId, Swarm, mdns::{Mdns, MdnsConfig, MdnsEvent},
    NetworkBehaviour, swarm::{NetworkBehaviourEventProcess, SwarmEvent}, tcp::TcpConfig,
    noise::{Keypair, NoiseConfig, X25519Spec}, identity,
    Transport, Multiaddr, gossipsub::{self, Gossipsub, GossipsubConfigBuilder, GossipsubEvent, MessageAuthenticity, TopicHash},
};
use crate::message::GossipMessage;
use futures::prelude::*;
use libp2p::gossipsub::Topic;
use std::hash::Hasher;
use std::collections::hash_map::DefaultHasher;

// Define a custom hasher for Topic
#[derive(Clone, Debug, Default)]
pub struct MyHasher;

impl libp2p::gossipsub::Hasher for MyHasher {
    fn hash(name: String) -> TopicHash {
        TopicHash::from_raw(&name)  // Use `from_raw` to directly convert from `&str` to `TopicHash`
    }
}

// Define a custom event to handle both Gossipsub and mDNS events
#[derive(Debug)]
enum GossipMdnsEvent {
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
}

impl From<GossipsubEvent> for GossipMdnsEvent {
    fn from(event: GossipsubEvent) -> Self {
        GossipMdnsEvent::Gossipsub(event)
    }
}

impl From<MdnsEvent> for GossipMdnsEvent {
    fn from(event: MdnsEvent) -> Self {
        GossipMdnsEvent::Mdns(event)
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "GossipMdnsEvent")]
struct GossipBehaviour {
    gossipsub: Gossipsub,
    mdns: Mdns,
}

impl NetworkBehaviourEventProcess<MdnsEvent> for GossipBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        if let MdnsEvent::Discovered(peers) = event {
            for (peer_id, _) in peers {
                println!("Discovered peer: {:?}", peer_id);
                self.gossipsub.add_explicit_peer(&peer_id);
            }
        }
    }
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for GossipBehaviour {
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let GossipsubEvent::Message { message, .. } = event {
            let received_message: GossipMessage = serde_json::from_slice(&message.data).unwrap();
            println!("Received message: {:?}", received_message);
        }
    }
}

pub struct Node {
    id: PeerId,
    swarm: Swarm<GossipBehaviour>,
}

impl Node {
    pub async fn new() -> Self {
        // Generate a random key pair for secure communication
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        println!("Node ID: {:?}", local_peer_id);

        // Configure Gossipsub with GossipsubConfigBuilder
        let gossipsub_config = GossipsubConfigBuilder::default()
            .build()
            .expect("Valid Gossipsub config");

        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
            .expect("Correct configuration");

        // Initialize mDNS with `await`
        let mdns = Mdns::new(MdnsConfig::default()).await.expect("Failed to create mDNS");

        // Configure noise encryption without AuthenticationMode
        let noise_keys = Keypair::<X25519Spec>::new().into_authentic(&local_key).expect("Noise key generation failed");

        let mut swarm = Swarm::new(
            TcpConfig::new()
                .upgrade(libp2p::core::upgrade::Version::V1)
                .authenticate(NoiseConfig::xx(noise_keys).into_authenticated())
                .multiplex(libp2p::yamux::YamuxConfig::default())
                .boxed(),
            GossipBehaviour { gossipsub, mdns },
            local_peer_id.clone(),
        );

        // Specify the topic using MyHasher as the hasher type
        let topic = Topic::<MyHasher>::new("gossip-topic");
        swarm.behaviour_mut().gossipsub.subscribe(&topic).unwrap();

        Node {
            id: local_peer_id,
            swarm,
        }
    }

    pub async fn run(&mut self) {  // Changed `self` to `&mut self`
        loop {
            match self.swarm.next().await.unwrap() {
                SwarmEvent::Behaviour(GossipMdnsEvent::Gossipsub(GossipsubEvent::Message { message, .. })) => {
                    let received_message: GossipMessage = serde_json::from_slice(&message.data).unwrap();
                    println!("Node {:?} received message {:?}", self.id, received_message);
                },
                SwarmEvent::Behaviour(GossipMdnsEvent::Mdns(MdnsEvent::Discovered(peers))) => {
                    for (peer_id, _) in peers {
                        println!("Discovered peer: {:?}", peer_id);
                        self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                _ => {}
            }
        }
    }

    pub fn send_gossip(&mut self, msg: GossipMessage) {
        let data = serde_json::to_vec(&msg).unwrap();
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(Topic::<MyHasher>::new("gossip-topic"), data)
            .unwrap();
    }
}
