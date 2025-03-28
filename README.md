# Elevator Network Control System - TTK4145

## Project Overview
This project implements a distributed elevator control system in Rust as part of the TTK4145 Real-Time Programming course at NTNU. The system is designed to manage multiple elevators in parallel with a focus on robustness and fault handling. It aims to guarantee no loss of service even during network disruptions, software crashes, or hardware failures. The system supports configurations with more than three elevators and four floors.

The core goals of the system are:
- **No single point of failure**: Any node can become the master.
- **Dynamic recovery**: System continues to function seamlessly through node crashes, disconnections, or restarts.
- **Redundancy-aware communication**: Robust handling of extreme packet loss.
- **Scalability**: Supports 3+ elevators and 4+ floors out of the box.


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

### Dynamic Master/Slave Role Allocation
Each node derives a unique ID based on its IP address. The **node with the lowest ID automatically becomes master**, with all others operating as slaves. Roles may change dynamically in response to failures or disconnections.

- **Master-node**:
  - Assigns tasks
  - Synchronizes state through UDP-broadcasts
  - Receives updates directly from slaves (via UDP direct messaging)

- **Slave-nodes**:
  - Manage their own elevator locally
  - Report status and hall requests to the master
  - Switch role if they detect master failure


**UDP broadcast**  
The master-node periodically broadcasts the latest system state (the worldview), which allows new nodes to discover and join the network. Each node listens for these broadcasts to stay synchronized with the master’s state.

**UDP direct messaging**  
The master-node listens for incoming UDP messages from the slave-nodes , containing elevator states. A simple acknowledgment scheme ensures reliable delivery by requiring slave-nodes  to wait for an acknowledgment of each message before sending a new one. This method allows the master-node to detect dead nodes by tracking the time since the last message from each slave. 

**Dynamic packet redundancy**  
To ensure reliable communication even with extreme packet loss, the system employs a dynamic redundancy mechanism. Before sending a packet, the sender calculates a redundancy factor, determining how many copies of the packet to send. This redundancy is controlled via a PID controller to adapt to varying network conditions, ensuring that enough messages are sent and acknowledged, even under high packet loss.

---

### Recommended Development Setup

To get the most out of this codebase, we highly recommend using **[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)** in **Visual Studio Code**.

With `rust-analyzer`, you can:
- Hover over functions, types, and modules to view inline documentation.
- Navigate the codebase more effectively with go-to definition and symbol search.
- Access auto-completion, type hints, and other helpful language features.

We've invested effort in writing thorough in-code documentation. Using a tool like `rust-analyzer` ensures you benefit from it while exploring or modifying the code.

---

## Visual Documentation

To enhance your understanding of the system's architecture and behavior, we have included several flowcharts in the `Flowcharts` directory. Below are the links to these flowcharts along with brief descriptions:


| Flowchart | Description |
|----------|-------------|
| [System Broadcast Mechanism](Flowcharts/Broadcast_direct.png) | Shows the master-slave communication via direct UDP and handling of sequence numbers and redundancy. |
| [UDP Broadcast Process](Flowcharts/UDP_broadcast.png) | Describes how the system listens to and sends broadcast packets for discovery and synchronization. |
| [Worldview Update Process](Flowcharts/update_wv.png) | Details the logic behind updating the `WorldView` when receiving new data or experiencing disconnects. |
| [Elevator FSM and Logic](Flowcharts/elev_fsm_and_logic.png) | Presents the local elevator’s control flow, timers, and decision-making based on events. |
| [Elevator State Machine Overview](Flowcharts/FSM.png) | A concise overview of high-level elevator states and transitions (e.g., moving, door open, error). |

We also have a picture of our amazing GUI:
[System-state Print Example](pic/ElevatorGUI.png)

