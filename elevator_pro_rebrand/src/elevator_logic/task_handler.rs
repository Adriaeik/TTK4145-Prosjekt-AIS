// use std::thread::sleep;
// use std::time::Duration;
// use tokio::sync::{mpsc, watch};

// // use crate::manager::task_allocator::ElevatorState;
// use crate::network::local_network;
// use crate::world_view::{self, ElevatorContainer, ElevatorStatus, TaskStatus};
// use crate::elevio::elev;


// pub async fn execute_tasks(wv_watch_rx: watch::Receiver<Vec<u8>>, update_elev_state_tx: mpsc::Sender<ElevatorStatus>, elevator: elev::Elevator){
//     let mut wv = world_view::get_wv(wv_watch_rx.clone());    

//     // loop{
//     //     let wv = utils::get_wv(chs.clone());
//     //     let wv_deser = world_view::deserialize_worldview(&wv);
//     //     world_view::print_wv(wv);

//     // }
//     let mut container: ElevatorContainer;
//     world_view::update_wv(wv_watch_rx.clone(), &mut wv).await;
//     container = world_view::extract_self_elevator_container(wv.clone());
//     world_view::update_wv(wv_watch_rx.clone(), &mut wv).await;
//     container = world_view::extract_self_elevator_container(wv.clone());
//     elevator.motor_direction(elev::DIRN_DOWN);
//     let mut last_state = ElevatorStatus::IDLE;
//     loop {
//         // let tasks_from_udp = utils::get_elev_tasks(chs.clone());
//         world_view::update_wv(wv_watch_rx.clone(), &mut wv).await;
//         container = world_view::extract_self_elevator_container(wv.clone());
//         let tasks_from_udp = container.task;

//         // utils::print_err(format!("last_floor: {}", container.last_floor_sensor));
//         // sleep(Duration::from_millis(50));
        
//         if let Some(task) = tasks_from_udp {
//             //utils::print_err(format!("TODO: {}, last_floor: {}", 0, container.last_floor_sensor));
//             if task.call.floor < container.last_floor_sensor {
//                 elevator.motor_direction(elev::DIRN_DOWN);
//                 if last_state != ElevatorStatus::DOWN {
//                     // utils::print_err("Starta execute tasks tråd".to_string());
//                     let _ = update_elev_state_tx.send(ElevatorStatus::DOWN).await;
//                     last_state = ElevatorStatus::DOWN;
//                 }
//             }
//             else if task.call.floor > container.last_floor_sensor {
//                 elevator.motor_direction(elev::DIRN_UP);
//                 if last_state != ElevatorStatus::UP {
//                     let _ = update_elev_state_tx.send(ElevatorStatus::UP).await;
//                     last_state = ElevatorStatus::UP;
//                 }
//             }
//             else {
//                 elevator.motor_direction(elev::DIRN_STOP);
//                 if last_state != ElevatorStatus::IDLE {
//                     let _ = update_elev_state_tx.send(ElevatorStatus::DOOR_OPEN).await;
//                     // open_door_protocol().await;
//                     sleep(Duration::from_millis(3000));
//                     let _ = update_elev_state_tx.send(ElevatorStatus::IDLE).await;
//                     last_state = ElevatorStatus::IDLE;
//                 }
//             }
//         } else {
//             elevator.motor_direction(elev::DIRN_STOP);
//             let _ = update_elev_state_tx.send(ElevatorStatus::IDLE).await;
//             last_state = ElevatorStatus::IDLE;
//             sleep(Duration::from_millis(100));
//         }
//     }
// }
