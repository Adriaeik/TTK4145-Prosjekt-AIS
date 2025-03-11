// use std::env;
// use std::net;
use log::{error, info, warn};
use std::process;
use std::thread::*;
use std::time::Duration;
// use serde::de;
// use uuid;

use crossbeam_channel as cbc;
use network_rust::udpnet;

use crate::config::Config;
use crate::distribute_orders;
use crate::elevator;
use crate::message::{self, Message};
use crate::order;
use crate::types;
use crate::types::Orders;

pub struct Network {
    config: Config,

    pub network_node_name: String,

    local_state: elevator::Elevator,
    peer_states: Vec<elevator::Elevator>,
    hall_orders: order::HallOrders,

    pub elevator_commands_tx: cbc::Sender<types::Orders>,

    pub peer_sender_tx: cbc::Sender<bool>,
    pub peer_receiver_rx: cbc::Receiver<udpnet::peers::PeerUpdate>,

    pub data_receiver_rx: cbc::Receiver<message::DataMessage>,
    pub data_send_tx: cbc::Sender<message::DataMessage>,
}

impl Network {
    /**
     * Start the UDP socket for peer discovery.
     */
    fn start_discovery_transmit(peer_port: u16, unique_name: String) -> cbc::Sender<bool> {
        let (peer_tx_enable_tx, peer_tx_enable_rx) = cbc::unbounded::<bool>();
        {
            // let id = process::id().to_string();
            spawn(move || {
                let result = udpnet::peers::tx(peer_port, unique_name, peer_tx_enable_rx);
                if result.is_err() {
                    error!(
                        "Failed to start peer discovery transmit: {}",
                        result.err().unwrap()
                    );
                    std::thread::sleep(Duration::from_secs(1));
                    process::exit(1); // crash program if creating the socket fails (`peers:tx` will always block if the initialization succeeds)
                }
            })
        };
        return peer_tx_enable_tx;
    }

    /**
     * Create receiver for peer discovery information.
     */
    fn start_discovery_receive(peer_port: u16) -> cbc::Receiver<udpnet::peers::PeerUpdate> {
        let (peer_update_tx, peer_update_rx) = cbc::unbounded::<udpnet::peers::PeerUpdate>();
        {
            spawn(move || {
                let result = udpnet::peers::rx(peer_port, peer_update_tx);
                if result.is_err() {
                    error!(
                        "Failed to start peer discovery receive: {}",
                        result.err().unwrap()
                    );
                    std::thread::sleep(Duration::from_secs(1));
                    process::exit(1); // crash program if creating the socket fails (`peers:rx` will always block if the initialization succeeds)
                }
            });
        }
        return peer_update_rx;
    }

    fn initate_channel_for_recieving_data(msg_port: u16) -> cbc::Receiver<message::DataMessage> {
        let (data_receiver_tx, data_receiver_rx) = cbc::unbounded::<message::DataMessage>();
        {
            spawn(move || {
                let result = udpnet::bcast::rx(msg_port, data_receiver_tx);
                if result.is_err() {
                    error!("Failed to start data receive: {}", result.err().unwrap());
                    std::thread::sleep(Duration::from_secs(1));
                    process::exit(1); // crash program if creating the socket fails (`bcast:rx` will always block if the initialization succeeds)
                }
            });
        }
        return data_receiver_rx;
    }

    fn initiate_channel_for_sending_data(msg_port: u16) -> cbc::Sender<message::DataMessage> {
        let (data_send_tx, data_send_rx) = cbc::unbounded::<message::DataMessage>();
        {
            spawn(move || {
                let result = udpnet::bcast::tx(msg_port, data_send_rx);
                if result.is_err() {
                    error!("Failed to start data send: {}", result.err().unwrap());
                    std::thread::sleep(Duration::from_secs(1));
                    process::exit(1); // crash program if creating the socket fails (`bcast:tx` will always block if the initialization succeeds)
                }
            });
        }
        return data_send_tx;
    }

    /**
     * Create a new network and begin peer discovery.
     * Also sets up the UDP socket for receiving and sending data messages.
     */
    pub fn new(
        config: Config,
        peer_port: u16,
        message_port: u16,
        unique_name: String,
        elevator_commands_tx: cbc::Sender<types::Orders>,
    ) -> Network {
        let network = Network {
            config,

            network_node_name: unique_name.clone(),

            local_state: elevator::Elevator::new(unique_name.clone()),
            peer_states: Vec::new(),
            hall_orders: order::HallOrders::new(),

            elevator_commands_tx: elevator_commands_tx,
            peer_sender_tx: Self::start_discovery_transmit(peer_port, unique_name),
            peer_receiver_rx: Self::start_discovery_receive(peer_port),
            data_receiver_rx: Self::initate_channel_for_recieving_data(message_port),
            data_send_tx: Self::initiate_channel_for_sending_data(message_port),
        };

        return network;
    }

    pub fn start_listening(mut self) {
        loop {
            cbc::select! {
                recv(self.peer_receiver_rx) -> received => {
                    let update = match received {
                        Ok(update) => update,
                        Err(e) => {
                            error!("Error receiving peer update: {e}");
                            continue;
                        }
                    };

                    // Check for new peers
                    if let Some(peer_name) = &update.new {
                        let is_local = peer_name == &self.network_node_name;
                        let peer = self.find_peer(&peer_name.to_string());
                        if !is_local && peer.is_none() {
                            self.new_peer_procedure(peer_name);
                        }
                    }

                    // Check for lost peers
                    if !update.lost.is_empty() {
                        for lost_peer in &update.lost {
                            if let Some(index) = self.peer_states.iter().position(|e| e.network_node_name == lost_peer.to_string()) { // Maybe move to a function `find_peer_index`
                                self.peer_states.remove(index);
                            }
                            info!("Lost elevator: {:#?}", lost_peer);
                        }
                    }
                }
                recv(self.data_receiver_rx) -> received => {
                    let message = match received {
                        Ok(message) => message,
                        Err(e) => {
                            error!("Error receiving data message: {e}");
                            continue;
                        }
                    };
                    match Self::infer_message_type(&message) {
                        message::MessageType::HallOrder(hall_order_message) => {
                            self.process_hall_order(hall_order_message, message.sender_node_name);
                        }
                        message::MessageType::CabOrder(cab_order_message) => {
                            self.process_cab_order(cab_order_message, message.sender_node_name);
                        }
                        message::MessageType::ElevatorEventMessage(elevator_event_message) => {
                            self.process_event(elevator_event_message, message.sender_node_name);
                        }
                        message::MessageType::Unknown => {
                            warn!("Unknown message type received.");
                        }
                    }
                }

                // Default
                default(Duration::from_millis(500)) => {
                    // debug!("Controller default");
                }
            }
        }
    }

    fn infer_message_type(message: &message::DataMessage) -> message::MessageType {
        if let Some(hall_order) = &message.hall_order_message {
            return message::MessageType::HallOrder(hall_order.clone());
        }
        if let Some(cab_order) = &message.cab_order_message {
            return message::MessageType::CabOrder(cab_order.clone());
        }
        if let Some(elevator_event) = &message.elevator_event_message {
            return message::MessageType::ElevatorEventMessage(elevator_event.clone());
        }
        message::MessageType::Unknown
    }

    fn process_hall_order(
        &mut self,
        hall_order: message::HallOrderMessage,
        sender_node_name: String,
    ) {
        info!(
            "Hall order received from {sender_node_name}: {:#?}",
            hall_order
        );

        // Run order distribution logic
        self.hall_orders
            .add_order(hall_order.direction, hall_order.floor);

        // Combine all peers including local state
        let all_elevators = {
            let mut peers = self.peer_states.to_vec();
            peers.push(self.local_state.clone());
            peers
        };

        // Calculate new order distribution
        let new_order_distribution = distribute_orders::distribute_orders(
            &self.config,
            all_elevators,
            self.hall_orders.clone(),
        );

        // Handle new order distribution
        let new_order_distribution = match new_order_distribution {
            Ok(distribution) => {
                info!("New order distribution: {:#?}", distribution);
                distribution
            }
            Err(e) => {
                error!("Failed to distribute orders: {e}");
                return;
            }
        };
        let local_node_name = &self.network_node_name;
        let local_order_distribution = match new_order_distribution.get(&self.network_node_name) {
            Some(local_distribution) => {
                info!(
                    "Local distribution ({local_node_name}): {:#?}",
                    local_distribution
                );
                local_distribution
            }
            None => {
                error!("Failed to get local distribution.");
                return;
            }
        };

        // Convert from HallOrders to Orders
        let orders = Orders::from_hall_orders(local_order_distribution, &self.config);
        self.elevator_commands_tx.send(orders).unwrap();
    }

    fn process_cab_order(&self, cab_order: message::CabOrderMessage, sender_node_name: String) {
        info!(
            "Cab order received from {sender_node_name}: {:#?}",
            cab_order
        );

        // Update elevator state
        // let elevator = elevator_states.iter_mut().find(|e| e.network_node_name == sender_node_name);
        // match elevator {
        //     Some(e) => {
        //         info!("Old elevator state: {:#?}", e);

        //         e.cab_orders.push(order::Order::new(cab_order.floor));

        //         info!("New elevator state: {:#?}", e);
        //     },
        //     None => {
        //         error!("Elevator with name {} not found in elevator_states", sender_node_name);
        //     }
        // }
    }

    fn process_event(&mut self, event: message::ElevatorEventMessage, sender_node_name: String) {
        // Check if the event is for the local elevator
        if self.network_node_name == sender_node_name {
            self.local_state.current_floor = Some(event.floor);
            self.local_state.behaviour = event.behaviour;
            self.local_state.direction = Some(event.direction);
            info!("Event - updated local state: {:#?}", self.local_state);
            return;
        }

        // Find or create a peer for this sender
        let peer = match self.find_peer_mut(&sender_node_name) {
            Some(existing_peer) => existing_peer,
            None => {
                info!("Event from unregistered peer {sender_node_name}");
                self.new_peer_procedure(&sender_node_name)
            }
        };

        // Update peer state
        peer.current_floor = Some(event.floor);
        peer.behaviour = event.behaviour;
        peer.direction = Some(event.direction);
        info!("Event - updated peer state: {:#?}", peer);
    }

    fn find_peer(&self, peer_name: &String) -> Option<&elevator::Elevator> {
        self.peer_states
            .iter()
            .find(|e| e.network_node_name == peer_name.to_string())
    }

    fn find_peer_mut(&mut self, peer_name: &String) -> Option<&mut elevator::Elevator> {
        self.peer_states
            .iter_mut()
            .find(|e| e.network_node_name == peer_name.to_string())
    }

    fn new_peer_procedure(&mut self, new_peer: &String) -> &mut elevator::Elevator {
        self.peer_states
            .push(elevator::Elevator::new(new_peer.to_string()));
        let new_elevator = self
            .peer_states
            .last_mut()
            .expect("peer_states should not be empty after push");
        info!("New peer: {:#?}", new_elevator);

        // Checklist - need to:
        // [ ] Send own state to new peer
        // [ ] Determine if new peer rebooted, or reconnected
        //   -> Reboot:    [ ] Send all orders to new peer
        //   -> Reconnect: [ ] Synchronize orders with new peer
        // [ ] Update order distribution

        // Publish own state to new peer (if not currently initializing). This may cause duplicate events to be received.
        if let (Some(current_floor), Some(direction)) =
            (self.local_state.current_floor, self.local_state.direction)
        {
            let event = message::ElevatorEventMessage {
                behaviour: self.local_state.behaviour,
                floor: current_floor,
                direction: direction,
            };

            if let Err(e) = self
                .data_send_tx
                .send(event.to_data_message(&self.network_node_name))
            {
                error!("Failed to send message: {:?}", e);
            }
        }

        new_elevator
    }
}
