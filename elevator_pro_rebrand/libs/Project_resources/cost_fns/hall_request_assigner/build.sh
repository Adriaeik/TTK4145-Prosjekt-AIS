#!/bin/bash

# Stoppar skriptet med ein gong viss noko feilar
set -ex

# Sørg for at vi har rettar til å køyre kompilert binær
sudo chmod +x d-json/jsonx.d || true

# Kompilerer prosjektet
sudo dmd main.d config.d elevator_algorithm.d elevator_state.d optimal_hall_requests.d d-json/jsonx.d -w -g -ofhall_request_assigner

# Sørg for at kompilert fil kan køyrast
sudo chmod +x hall_request_assigner
