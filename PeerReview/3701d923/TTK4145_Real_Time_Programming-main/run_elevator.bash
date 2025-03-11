#!/usr/bin/env bash

# If no ID is supplied, default to 1
ID=$1
if [ -z "$ID" ]; then
  ID=1
fi

# Set up environmental variables
ELEVATOR_NETWORK_ID=$ID
ELEVATOR_HARDWARE_PORT=15657
NUMBER_FLOORS=4

# Run the process
sudo -E ELEVATOR_NETWORK_ID=$ELEVATOR_NETWORK_ID \
       ELEVATOR_HARDWARE_PORT=$ELEVATOR_HARDWARE_PORT \
       NUMBER_FLOORS=$NUMBER_FLOORS \
       cargo run --bin elevator_process_pair