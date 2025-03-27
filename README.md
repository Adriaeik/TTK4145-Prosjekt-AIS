# Elevator Network Control System - TTK4145

## Project Overview
This project implements a distributed elevator control system in Rust as part of the TTK4145 Real-Time Programming course at NTNU. The system is designed to manage multiple elevators in parallel with a focus on robustness and fault handling. It aims to guarantee no loss of service even during network disruptions, software crashes, or hardware failures. The system supports configurations with more than three elevators and four floors.



### Requirements
**Service guarantee**:  
- Once a button light is turned on, an elevator should arrive at that floor.  

**No calls are lost**:
- System failures: 
    - Losing network connection entirely
    - Software crashes
    - Doors that wont close
    - Powerloss, both to the elevators motor and the machine that controls it.
    - Note: any amount of packetloss less than 100% is not considered a failure 

- No calls should be lost under system failure, which implies:
    - Cab calls should be executed once service to the elevator is restored after a failure
    - Time used to compensate for failures should be reasonable (within seconds)

**Synchronisation**:
- Hall buttons on all workspaces should be able to summon any elevator
- Hall lights on all workspaces should show the same thing


### Assumptions
- At least one elevator in the system is always operational and not in a failure state


## Our Solution

**Dynamic Master/Slave-nodes**  
Each node in the network is assigned an ID based on its IP address. The master-node is dynamically selected as the node with the lowest ID. The system operates on a hybrid master-slave model, where certain tasks (like handling cab calls) are executed by the slave-nodes .
- The master-node manages system-wide coordination and delegates tasks via UDP-broadcast.
- The slave-nodes  performs local elevator handling and sends status updates to the master-node via direct UDP messaging

**UDP broadcast**  
The master-node periodically broadcasts the latest system state (the worldview), which allows new nodes to discover and join the network. Each node listens for these broadcasts to stay synchronized with the master’s state.

**UDP direct messaging**  
The master-node listens for incoming UDP messages from the slave-nodes , containing elevator states. A simple acknowledgment scheme ensures reliable delivery by requiring slave-nodes  to wait for an acknowledgment of each message before sending a new one. This method allows the master-node to detect dead nodes by tracking the time since the last message from each slave. 

**Dynamic packet redundancy**  
To ensure reliable communication even with extreme packet loss, the system employs a dynamic redundancy mechanism. Before sending a packet, the sender calculates a redundancy factor, determining how many copies of the packet to send. This redundancy is controlled via a PID controller to adapt to varying network conditions, ensuring that enough messages are sent and acknowledged, even under high packet loss.


### Recommended Development Setup

To get the most out of this codebase, we highly recommend using **[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)** in **Visual Studio Code**.

With `rust-analyzer`, you can:
- Hover over functions, types, and modules to view inline documentation.
- Navigate the codebase more effectively with go-to definition and symbol search.
- Access auto-completion, type hints, and other helpful language features.

We've invested effort in writing thorough in-code documentation. Using a tool like `rust-analyzer` ensures you benefit from it while exploring or modifying the code.