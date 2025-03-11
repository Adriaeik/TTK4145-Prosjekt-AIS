# TTK4145_Real_Time_Programming
Collaborative real-time software development project in Rust, focusing on synchronization, scheduling, fault tolerance, and consistency in a POSIX-based environment (Portable Operating System Interface)



<br>



## Running the Elevator System
Some commands can be long, so we provide macros to start the whole system.
You can modify the `.bash` files for further customization (e.g., custom ports, floor counts, or multiple elevators ect...).


```
$ cd elevator-server
$ ./run_server.bash
```
```
$ ./run_database.bash <ID>
```
```
$ ./run_manager.bash <ID>
```
```
$ ./run_elevator.bash <ID>
```

_NOTE: If you run multiple instances on the same network, each process must have a unique ID (ID is a `uint` datatype)_

You can also run these commands directly. See the chapters below for more details.
For a fault tolerant concurrent distributed system, it is recommended to run 2+ of database and manager nodes, whilst elevator node should be the same amount as physical elevators on the hardware (although 2 elevator nodes can still control just 1 hardware elevator, it is impractical)




<br>



## Good commands to know
```$ cargo run```: Builds and runs the default binary target in your project.

```$ cargo check```: Quickly checks your code for syntax errors and type correctness without producing an executable or build artifacts.

```$ cargo build```: Compiles the project without running it, storing the build artifacts in the target directory.

```$ cargo clean```: Removes all build artifacts, clearing the target directory to start fresh.



<br>



## Running Bootled Data Distribution Service (DDS) Node
This program demonstrates how running multiple database instances with unique `DATABASE_NETWORK_ID` in the same local network creates a basic, bootleg *Data Distribution Service (DDS)*. In this setup, nodes handle data synchronization and distribution, ensuring robustness. For example, cutting one database, disconnecting the publisher, adding a late-joining subscriber, toggling the network, or delaying a node's response lets you observe how the system adapts to disruptions.

Because the system uses libraries (e.g. for ICMP diagnostics) that require root privileges, every process must be run with sudo. Also, each instance needs a unique `DATABASE_NETWORK_ID` for leader election (lower IDs mean higher priority).

In addition we must specify what type of elevators are on our network that the database should track, back their data and redistribute the data. We specify this is a string array format in `ELEVATOR_NETWORK_ID_LIST` parameter.

In process pair mode, if the database node exits or loses network connection, the monitor restarts it automatically. To run a database node, execute:

To run the database, execute the following command:

```
$ sudo -E DATABASE_NETWORK_ID=<ID> ELEVATOR_NETWORK_ID_LIST="[<ID 1>,<ID 2>,...,<ID N>]" NUMBER_FLOORS=<NUMBER FLOORS> cargo run --bin database_process_pair
```



<br>



## Running Manager Node
The Manager Node is responsible for assigning hall requests to elevators based on the cost function algorithm. It listens for elevator status updates, runs the cost algorithm, and sends assignment commands back to the system.

Each manager node requires a unique `MANAGER_ID` and must be aware of all elevators in the system by specifying `ELEVATOR_NETWORK_ID_LIST`. For fault tolerance, multiple manager nodes can be run, and leader election determines which node takes control for synchronized data.

First build the cost function algorithm executable:
```
$ cd Project-resources/cost_fns/hall_request_assigner
$ ./build.sh
$ cd ../../..
```

Now run the manager node:
```
$ sudo -E MANAGER_ID=<ID> ELEVATOR_NETWORK_ID_LIST="[<ID 1>,<ID 2>,...,<ID N>]" cargo run --bin manager_process_pair
```



<br>



## Running Elevator Node
The elevator module implements the elevator control system in a distributed environment. Each elevator node requires its own unique `ELEVATOR_NETWORK_ID` and must be assigned a dedicated hardware port via `ELEVATOR_HARDWARE_PORT` (e.g. the elevator hardware listens on that port). Running with the proper privileges ensures the hardware (or its simulator) is correctly accessed and that the elevator state is synchronized over the network.

To run an elevator node, use:

```
$ sudo -E ELEVATOR_NETWORK_ID=<ID> ELEVATOR_HARDWARE_PORT=<PORT> NUMBER_FLOORS=<NUMBER FLOORS> cargo run --bin elevator_process_pair
```



<br>



## Running examples
### Multithreading
Demonstrates a basic multi-threaded process using Tokio's asynchronous runtime, combining cooperative multitasking with a shared resource protected by an Arc\<Mutex>. This example highlights how tasks can safely increment and decrement a shared value concurrently while showcasing Tokio's efficiency and scalability for distributed or concurrent programming.

```
$ cargo run --example multi_thread_process
```

### Distributed Network
Example demonstrating a distributed messaging system using Zenoh for efficient data communication and Tokio for asynchronous multithreading. The publisher sends data to specific topics, while the subscriber receives and processes messages from those topics, showcasing a complete publish-subscribe pattern in a networked environment.

```
$ cargo run --example publisher_node
```

```
$ cargo run --example subscriber_node
```

### Synchronizing nodes
This system implements a decentralized, fault-tolerant network using Rust. It focuses on leadership election, message synchronization, and distributed communication. Random numbers are published to a temporary topic by a separate process, which the leader node subscribes to and re-broadcasts to a storage topic. Nodes use heartbeat signals to detect leader failures, dynamically electing a new leader based on node ID priorities. The leader ensures the synchronization of received data across the system by broadcasting it in real time. Subscribers can listen to the storage topic to receive the latest synchronized data. This architecture ensures consistency, minimizes downtime, and supports dynamic fault recovery in a distributed environment. It demonstrates how real-time systems can maintain reliability and responsiveness despite network or node failures.

```
$ cargo run --example synchronized_sender
```

```
$ cargo run --example synchronized_database1
```

```
$ cargo run --example synchronized_database2
```

```
$ cargo run --example synchronized_receiver
```

### Multiple Databases
This is a small example demonstrating how chaining multiple databases can enhance the robustness of our system, essentially making a bootled Distributed Data System (DDS). We will send multiple topics from different sources and receive the data. However, without additional measures, if one of the nodes fails, the signal is lost, or if a node rejoins the network, it lacks information about the current system state. Managing synchronization and data distribution on each node is complex. That is why using a Distributed Data System (DDS) is a better approach, where the database nodes handle everything related to data, allowing us to focus on more valuable tasks without worrying about data distribution.

In this example, we will start two database nodes, one multi-publisher node, and one multi-receiver node. To test the system's resilience, try disrupting it by:
- Cutting one of the databases.
- Disconnecting the publisher or having the subscriber join late to request data.
- Turning the network on and off.
- Desynchronizing the system by, for instance, simulating an extended delay in one node's response.

As you can see, running the database module requires `sudo`. Refer to the section **Running Bootled Data Distribution Service (DDS)** above for more details.

To run the system, execute the following commands:

```
$ sudo DATABASE_ID=1 cargo run --bin database_process_pair
```

```
$ sudo DATABASE_ID=2 cargo run --bin database_process_pair
```

```
$ sudo cargo run --example multi_pub
```

```
$ sudo cargo run --example multi_sub
```

### Other examples 
To run any other examples just run the following command format:

```
$ cargo run --example <EXAMPLE_NAME>
```



