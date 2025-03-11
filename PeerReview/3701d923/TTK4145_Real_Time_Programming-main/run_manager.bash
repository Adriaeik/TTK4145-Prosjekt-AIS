#!/usr/bin/env bash

# Environment setup
# If no ID is supplied, default to 1
ID=$1
if [ -z "$ID" ]; then
  ID=1
fi

# Set up environmental variables
MANAGER_ID=$ID
ELEVATOR_NETWORK_ID_LIST="[1,2,3]"

# Before running the process we need to compile the cost function algorithm
cd Project-resources/cost_fns/hall_request_assigner
./build.sh
cd ../../..

# Run the process
sudo -E MANAGER_ID=$MANAGER_ID \
        ELEVATOR_NETWORK_ID_LIST=$ELEVATOR_NETWORK_ID_LIST \
        cargo run --bin manager_process_pair