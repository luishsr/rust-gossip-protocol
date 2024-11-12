mod node;
mod message;

use tokio::sync::Mutex;
use node::Node;
use message::GossipMessage;
use tokio::time::{sleep, Duration};
use std::sync::Arc;


#[tokio::main]
async fn main() {
    // Wrap `Node` in an `Arc<tokio::sync::Mutex<_>>` to share safely across tasks
    let node = Arc::new(Mutex::new(Node::new().await));

    // Spawn the node's event loop in its own task
    let node_for_run = Arc::clone(&node);
    tokio::spawn(async move {
        let mut node = node_for_run.lock().await;  // Await the lock acquisition
        node.run().await;  // Call `run` with a mutable reference
    });

    // Give nodes some time to discover each other via mDNS
    sleep(Duration::from_secs(2)).await;

    // Send a gossip message from the main task
    let msg = GossipMessage {
        id: 1,
        payload: "Hello from the network!".into(),
    };

    // Lock the node to send a gossip message
    node.lock().await.send_gossip(msg);

    // Keep the main task alive to allow nodes to continue communicating
    loop {
        sleep(Duration::from_secs(10)).await;
    }
}