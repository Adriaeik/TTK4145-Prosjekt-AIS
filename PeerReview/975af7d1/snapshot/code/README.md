# Code snapshot

## Architecture

### Networking
The elevators communicate on a peer-to-peer network. When a message is sent it is broadcasted to all other known peers. It is then acknowledged (not implemented). If all known peers acknowledge, the elevator that sent the message will not do anything more and after a timeout all elevators receiving the message will carry out the action. If, however, it does not receive acknowledgment from all peers it will send a revert to all nodes (hopefully within the timeout) and carry out fault handling logic (this is not implemented). Note that an elevator always sends any message to itself also.

### Reassigning orders
All elevators run the single elevator program, reassignment of all elevators orders happens on every node when any process for a successfully sent and received message is carried out. That is: after a message is acknowledged and timed out (and thus assumed to be known by everyone) and system-state has been mutated accordingly, the orders will be redistributed. Under the assumption that the state of the system was originally agreed upon by all elevators and that the message was received by everyone, all elevators should have the same state and thus also come to the same conclusion about the order-distribution. 

## Running the project
You need to compile the hall request assigner and add it to src/binaries/hall_request_assigner_${OS} OS=macos, linux to run the code.
