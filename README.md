# Elevator Network Control System

## Project Overview
This project implements a distributed elevator control system in Rust as part of the TTK4145 Real-Time Programming course at NTNU. The system is designed to manage multiple elevators in parallel with a focus on robustness, fault handling, and optimal task distribution.

## Current Status
The project is currently in an unfinished state but has made significant progress in key areas. The core infrastructure for a distributed elevator control system has been established, enabling communication between multiple elevators over a network. The system successfully assigns tasks (In a very un-optimal way), updates elevator states, and handles failures to some extent. However, several critical features are still under development, and improvements are needed to meet the robustness and efficiency requirements outlined in the project specification.

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
5. **For information about arguments**: `cargo run -- help`

## Next Steps
The project is still under development. If you have suggestions or spot issues, let us know.

# TTK4145-Prosjekt-AIS
 This is the main project

Packet-loss:
Oversimplified packet loss script.
This program intentionally has significant limitations. Use 'iptables' directly for advanced operations.

Remember to run this program with 'sudo'

Options:
Either long or short options are allowed
    --ports -p  <network ports> (comma-separated)
        The ports to apply packet loss to
    --name  -n  <executablename>        
        Append ports used by any executables matching <executablename> to the ports list
    --rate  --probability  -r   <rate> (floating-point value between 0 and 1, inclusive)
        The packet loss rate. Use 1 for "disconnect".
        Omitting this argument will set the rate to 0.0
    --flush -f
        Remove all packet loss rules
        
Examples:
    sudo packetloss -f
        Removes all packet loss rules, disabling packet loss
        
    sudo packetloss -p 12345,23456,34567 -r 0.25
        Applies 25% packet loss to ports 12345, 23456, and 34567
        
    sudo packetloss -n elevator_pro -r 0.25
        Applies 25% packet loss to all ports used by all programs named "elevator_pro"
        
    sudo packetloss -p 12345 -n executablename -r 0.25
        Also applies 25% packet loss to port 12345

    sudo packetloss -n executablename -f
        Lists ports used by "executablename", but does not apply packet loss
   



Lese argumenter: cargo run -- <kommandoer>
```rs
pub fn hent_sjefpakke() -> Result<SjefPakke, &'static str> {
    use std::env;
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let command = args[1].to_lowercase();
        
        let rolle = match command.as_str() {
            "backup" => Rolle::BACKUP,
            "master" => Rolle::SLAVE,
            _ => return Err("Ugyldig kommando. Bruk 'backup' eller 'master'."),
        };

        Ok(SjefPakke {rolle})
    } else {
        Err("Bruk: <program> <kommando>")
    }
}
```
