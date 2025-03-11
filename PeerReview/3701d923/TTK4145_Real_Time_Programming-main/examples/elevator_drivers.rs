use std::collections::HashMap;

use std::sync::Arc;
use tokio::spawn;
use tokio::sync::{mpsc, Mutex};
use tokio::task::yield_now;
use tokio::time::Duration; // Importing yield_now for cooperative multitasking

// Import necessary drivers and types
use elevator_system::elevator_io::{data, driver};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let addr = "localhost:15657";
    let num_floors = 4;
    // * NOTE: period must be < 100 ms otherwise the latency to big and we might miss the button press or other key events
    let period: u64 = 100; // ms

    // Initialize the elevator
    let elevator = driver::Elevator::init(addr, num_floors).await;
    println!("Elevator initialized:\n{:#?}", elevator);

    // 6 - READING: Button orders testing (START) ====================================================================================================
    // Shared states for cab calls and floor calls
    let cab_calls = Arc::new(Mutex::new((0..num_floors).map(|floor| (floor, false)).collect::<HashMap<_, _>>()));
    let floor_calls = Arc::new(Mutex::new((0..num_floors).map(|floor| (floor, (false, false))).collect::<HashMap<_, _>>()));

    // Create a channel for button call updates
    let (button_tx, mut button_rx) = mpsc::channel(32);

    // Poll button calls and send updates
    let elevator_clone = elevator.clone();
    spawn(async move {
        data::call_buttons(elevator_clone, button_tx, Duration::from_millis(period)).await;
    });

    // Process button call updates and update states
    let cab_calls_clone = Arc::clone(&cab_calls);
    let floor_calls_clone = Arc::clone(&floor_calls);
    spawn(async move {
        while let Some(button) = button_rx.recv().await {
            match button.call {
                2 => {
                    // CAB button
                    let mut cab = cab_calls_clone.lock().await;
                    cab.insert(button.floor, true);
                    println!("Cab call updated: Floor {}", button.floor);
                }
                1 => {
                    // DOWN button
                    let mut floors = floor_calls_clone.lock().await;
                    let entry = floors.entry(button.floor).or_insert((false, false));
                    entry.0 = true;
                    println!("Floor call updated: Floor {}, DOWN pressed", button.floor);
                }
                0 => {
                    // UP button
                    let mut floors = floor_calls_clone.lock().await;
                    let entry = floors.entry(button.floor).or_insert((false, false));
                    entry.1 = true;
                    println!("Floor call updated: Floor {}, UP pressed", button.floor);
                }
                _ => {} // Ignore invalid types
            }
        }
    });

    // Periodically print cab and floor call states
    let cab_calls_monitor = Arc::clone(&cab_calls);
    let floor_calls_monitor = Arc::clone(&floor_calls);
    spawn(async move {
        loop {
            let cab = cab_calls_monitor.lock().await;
            let floors = floor_calls_monitor.lock().await;

            println!("Cab calls: {:?}", *cab);
            println!("Floor calls: {:?}", *floors);

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
    // 6 - READING: Button orders testing (STOP) ====================================================================================================

    // 7 - READING: Floor sensor testing (START) ====================================================================================================
    // Shared state for the current floor
    let current_floor = Arc::new(Mutex::new(None::<u8>)); // Initially no floor detected

    // Create a channel for floor sensor updates
    let (floor_tx, mut floor_rx) = mpsc::channel(32);

    // Poll floor sensor and send updates
    let elevator_clone = elevator.clone();
    spawn(async move {
        data::floor_sensor(elevator_clone, floor_tx, Duration::from_millis(period)).await;
    });

    // Process floor updates and print state
    let current_floor_clone = Arc::clone(&current_floor);
    spawn(async move {
        while let Some(floor) = floor_rx.recv().await {
            let mut floor_data = current_floor_clone.lock().await;
            *floor_data = Some(floor); // Save the current floor
            println!("Current floor: {}", floor);
        }
    });
    // 7 - READING: Floor sensor testing (STOP) ====================================================================================================

    // 8 - READING: Stop button testing (START) ==================================================================================================
    // Shared state for stop button light
    let stop_button_state = Arc::new(Mutex::new(false)); // Initially, stop button is not pressed

    // Control stop button light
    let elevator_clone = elevator.clone();
    let (stop_tx, mut stop_rx) = mpsc::channel(32);

    // Spawn a task to poll the stop button state
    spawn(async move {
        data::stop_button(elevator_clone.clone(), stop_tx, Duration::from_millis(period)).await;
    });
    // 8 - READING: Stop button testing (STOP) ==================================================================================================

    // 9 - READING: Obstruction switch testing (START) ==================================================================================================
    // Control obstruction switch state
    let obstruction_state = Arc::new(Mutex::new(false)); // Shared state for obstruction switch
    let obstruction_state_clone = Arc::clone(&obstruction_state);
    let elevator_clone = elevator.clone();
    let (obstruction_tx, mut obstruction_rx) = mpsc::channel(32);

    // Spawn a task to poll the obstruction switch state
    spawn(async move {
        data::obstruction(elevator_clone.clone(), obstruction_tx, Duration::from_millis(period)).await;
    });

    // Spawn a task to update the obstruction state
    spawn(async move {
        while let Some(is_active) = obstruction_rx.recv().await {
            {
                let mut obstruction = obstruction_state_clone.lock().await;
                *obstruction = is_active; // Update the obstruction switch state
            }
            println!("Obstruction switch state updated: {}", if is_active { "Active" } else { "Inactive" });
        }
    });

    // Spawn a task to periodically print the obstruction state
    let obstruction_state_monitor = Arc::clone(&obstruction_state);
    spawn(async move {
        loop {
            let obstruction = obstruction_state_monitor.lock().await;
            println!("Obstruction switch state: {}", if *obstruction { "Active" } else { "Inactive" });
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
    // 9 - READING: Obstruction switch testing (STOP) ==================================================================================================

    // 1 - WRITING: Motor direction testing (START) ==================================================================================================
    // Control motor direction based on current floor
    let current_floor_motor = Arc::clone(&current_floor);
    let elevator_clone = elevator.clone();
    spawn(async move {
        let mut direction: u8 = 1; // Start by moving up

        loop {
            let floor = {
                let current_floor = current_floor_motor.lock().await;
                *current_floor
            };

            match floor {
                Some(floor) => {
                    if floor == 0 {
                        direction = 1; // Switch to moving up
                        println!("Reached the lowest floor. Changing direction to UP.");
                    } else if floor == num_floors - 1 {
                        direction = 255; // Switch to moving down
                                         // Why 255 and not -1?
                                         // because driver requires data in uint8_t
                                         // ANd uint can only take positive values, we use 255 to say -1 basically
                                         // cursed X-X
                        println!("Reached the highest floor. Changing direction to DOWN.");
                    }
                }
                None => println!("No floor detected. Holding position."),
            }

            // Set motor direction based on the current direction
            elevator_clone.motor_direction(if direction == 1 { 1 } else { 255 }).await;

            tokio::time::sleep(Duration::from_millis(period)).await; // Update motor direction periodically
        }
    });
    // 1 - WRITING: Motor direction testing (STOP) ==================================================================================================

    // 2 - WRITING: Button order light testing (START) ==================================================================================================
    // Control order button lights for cab calls and floor calls
    let cab_calls_light = Arc::clone(&cab_calls);
    let floor_calls_light = Arc::clone(&floor_calls);
    let elevator_clone = elevator.clone();
    spawn(async move {
        loop {
            {
                // Update cab call lights
                let cab = cab_calls_light.lock().await;
                for (floor, active) in cab.iter() {
                    elevator_clone.call_button_light(*floor, 2, *active).await;
                }

                // Update floor call lights
                let floors = floor_calls_light.lock().await;
                for (floor, (down, up)) in floors.iter() {
                    elevator_clone.call_button_light(*floor, 1, *down).await;
                    elevator_clone.call_button_light(*floor, 0, *up).await;
                }
            }
            tokio::time::sleep(Duration::from_millis(period)).await; // Periodic update
        }
    });
    // 2 - WRITING: Button order light testing (STOP) ==================================================================================================

    // 3 - WRITING: Floor indicator testing (START) ==================================================================================================
    // Control floor indicator light
    let current_floor_light = Arc::clone(&current_floor);
    let elevator_clone = elevator.clone();
    spawn(async move {
        loop {
            let floor = {
                let current_floor = current_floor_light.lock().await;
                *current_floor
            };

            if let Some(floor) = floor {
                elevator_clone.floor_indicator(floor).await;
            }

            tokio::time::sleep(Duration::from_millis(period)).await; // Periodic update light
        }
    });
    // 3 - WRITING: Floor indicator testing (STOP) ==================================================================================================

    // 4 - WRITING: Door open light testing (START) ==================================================================================================
    // Shared state for door open state
    let door_open = Arc::new(Mutex::new(false)); // Initially door is closed

    // Simulate door open/close events with door light control
    let door_open_clone = Arc::clone(&door_open);
    let elevator_clone = elevator.clone();
    spawn(async move {
        loop {
            {
                let mut door = door_open_clone.lock().await;
                *door = !*door; // Toggle door state
                if *door {
                    println!("Door opened");
                    elevator_clone.door_light(true).await;
                } else {
                    println!("Door closed");
                    elevator_clone.door_light(false).await;
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await; // Simulate door operation every 5 seconds
        }
    });

    // Print door state periodically
    let door_open_monitor = Arc::clone(&door_open);
    spawn(async move {
        loop {
            let door = door_open_monitor.lock().await;
            println!("Door state: {}", if *door { "Open" } else { "Closed" });
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    // 4 - WRITING: Door open light testing (STOP) ==================================================================================================

    // 5 - WRITING: Stop button light testing (START) ==================================================================================================
    // Spawn a task to update the stop button state and light
    let stop_button_state_clone = Arc::clone(&stop_button_state);
    let elevator_clone = elevator.clone();
    spawn(async move {
        while let Some(is_pressed) = stop_rx.recv().await {
            {
                let mut stop_state = stop_button_state_clone.lock().await;
                *stop_state = is_pressed; // Update the stop button state
            }
            elevator_clone.stop_button_light(is_pressed).await;
            println!("Stop button light updated: {}", if is_pressed { "ON" } else { "OFF" });
        }
    });

    // Spawn a task to periodically print the stop button state
    spawn(async move {
        loop {
            let stop_state = stop_button_state.lock().await;
            println!("Stop button state: {}", if *stop_state { "Pressed" } else { "Not pressed" });
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
    // 5 - WRITING: Stop button light testing (STOP) ==================================================================================================

    loop {
        yield_now().await;
    }
}
