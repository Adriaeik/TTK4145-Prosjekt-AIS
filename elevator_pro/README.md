# Elevator Network Control System

## Project Overview
This project implements a distributed elevator control system in Rust as part of the TTK4145 Real-Time Programming course at NTNU. The system is designed to manage multiple elevators in parallel with a focus on robustness, fault handling, and optimal task distribution.

## Current Status
The project is currently in an unfinished state but has made significant progress in key areas. The core infrastructure for a distributed elevator control system has been established, enabling communication between multiple elevators over a network. The system successfully assigns tasks, updates elevator states, and handles failures to some extent. However, several critical features are still under development, and improvements are needed to meet the robustness and efficiency requirements outlined in the project specification.

Our current implementation includes a basic master-slave architecture where one elevator acts as the master, managing and distributing tasks among available elevators. This system ensures that task assignments are handled even if an elevator fails. The communication layer is functional, with UDP and TCP handling state updates and task delegation. However, there are gaps in fault tolerance, task allocation efficiency, and system recovery that need to be addressed before the project can be considered complete.
Below is a summary of what has been done, what needs improvement, and what remains to be implemented.

### Implemented Features
✅ **Distributed Elevator Network**: Elevators communicate via TCP and UDP to update state and handle tasks.

✅ **Master-Slave Handling**: One elevator is assigned as master and coordinates tasks. If an elevator fails, another takes over.

✅ **Basic Task Distribution**: Orders are assigned to elevators, but the distribution algorithm needs improvement.

✅ **Network Communication Handling**: Packets are exchanged between master and slaves, and the system's worldview is continuously updated.

### Areas for Improvement
🔄 **Improved Task Distribution**: The cost function for task assignment needs optimization to ensure faster and more efficient elevator movement.

🔄 **Better Fault Handling**: If the master elevator dies while a TCP message is being sent, the new master must still receive the information.

🔄 **Elevator Light Control**: Button lights are not yet implemented, which is a requirement that must be addressed.

🔄 **Local Backup for Master/Slave**: Each unit should maintain an inactive clone of the program state to take over in case of a crash or manual termination (Ctrl+C).

### Remaining Tasks
🔜 **Implement local backup for each elevator**

🔜 **Ensure TCP messages are redirected to a new master if the current one fails**

🔜 **Complete implementation of elevator light handling**

🔜 **Optimize task distribution with a better cost function**

## Addressing the Main Project Requirements
The project's goal is to create a robust system where:
- No orders are lost, even in the case of network failures or crashes.
- The system efficiently manages multiple elevators in parallel.
- Elevators respond correctly to user input and execute tasks reliably.
- The system tolerates failures and automatically restores functionality.

The foundation for the system is in place, but further development is needed to fully meet all project requirements.

## Running the Code
To run the system, follow these steps:
1. **Install Rust** if not already installed.
2. **Clone the repository**: `git clone https://github.com/Adriaeik/TTK4145-Prosjekt-AIS`
3. **Go to the directory**: `cd TTK4145-Prosjekt-AIS/elevator_pro`
4. **Run an elevator instance**: `cargo run`

## Next Steps
The project is still under development. If you have suggestions or spot issues, let us know.


