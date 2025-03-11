#!/usr/bin/env bash

# If no ID is supplied, default to 1
ID=$1
if [ -z "$ID" ]; then
  ID=1
fi

# Set up environmental variables
DATABASE_NETWORK_ID=$ID
ELEVATOR_NETWORK_ID_LIST="[1,2,3]"
NUMBER_FLOORS=4

# Run the process
sudo -E DATABASE_NETWORK_ID=$DATABASE_NETWORK_ID \
       ELEVATOR_NETWORK_ID_LIST=$ELEVATOR_NETWORK_ID_LIST \
       NUMBER_FLOORS=$NUMBER_FLOORS \
       cargo run --bin database_process_pair
