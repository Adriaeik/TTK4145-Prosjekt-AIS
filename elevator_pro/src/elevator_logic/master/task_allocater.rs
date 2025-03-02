// -----------------------------------------------------------------------------
// Kalkulerer ein "kostnad" for kor godt ein heis kan ta imot eit eksternt kall
// -----------------------------------------------------------------------------
fn kalkuler_kostnad(elev: &ElevatorStatus, call: &CallButton) -> u32 {
    // Basiskostnad er avstanden i etasjar
    let diff = if elev.current_floor > call.floor {
        elev.current_floor - call.floor
    } else {
        call.floor - elev.current_floor
    } as u32;
    let mut kostnad = diff;
    
    // Legg til ekstra kostnad dersom heisens retning ikkje stemmer med kallretninga
    match (elev.direction, call.call) {
        // Om heisen køyrer opp og kall er UP, og heisen er under kall-etasjen
        (Direction::Up, CallType::UP) if elev.current_floor <= call.floor => { }
        // Om heisen køyrer ned og kall er DOWN, og heisen er over kall-etasjen
        (Direction::Down, CallType::DOWN) if elev.current_floor >= call.floor => { }
        // Om heisen er idle er det optimalt
        (Direction::Idle, _) => { }
        // I alle andre tilfelle legg til ein straff
        _ => {
            kostnad += 100;
        }
    }
    
    // Legg til kostnad basert på talet på allereie tildelte oppgåver
    kostnad += (elev.tasks.len() as u32) * 10;
    
    kostnad
}

// -----------------------------------------------------------------------------
// Funksjon som tildeler ein oppgåve til rett heis
//
// - For INSIDE kall: finn heisen med samsvarande elev_id (forutsatt at han ikkje er offline).
// - For eksterne kall (UP/DOWN): vel heisen med lågaste kostnad.
// -----------------------------------------------------------------------------
pub fn tildele_oppgave(elevators: &[ElevatorStatus], call: CallButton) -> Option<u8> {
    // Dersom kalltypen er INSIDE, skal oppgåva gå til den spesifikke heisen
    if call.call == CallType::INSIDE {
        return elevators.iter()
            .find(|e| e.elevator_id == call.elev_id && !e.offline)
            .map(|e| e.elevator_id);
    }
    
    // For eksterne kall: iterer gjennom alle heisar som ikkje er offline
    let mut beste_id = None;
    let mut beste_kostnad = u32::MAX;
    
    for elev in elevators.iter().filter(|e| !e.offline) {
        let kost = kalkuler_kostnad(elev, &call);
        if kost < beste_kostnad {
            beste_kostnad = kost;
            beste_id = Some(elev.elevator_id);
        }
    }
    
    beste_id
}
