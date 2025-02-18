

#[tokio::main]
async fn main() {

    let udp_listener = start_udp_listener().await; //Oppdaterer WV om lavere ID blir med

    let udp_broadcaster = start_udp_broadcaster(); //Broadcaster WV om du er lavest ID

    let tcp_task = start_tcp_listener(); //TCP connection mellom master eller slaver

    let heis_logikk_task = start_process(); //Selve kjøre-mekanismene


}


// Kanskje lage is_master = min_id == lavest_id_i_wv i en global multithread vareabel
// Så kan worldview-thread oppdatere den (låse + skrive) og alle kan lese av den uten å låse (så wv ikke blir låst hver gang den skal beregnes)
pub async fn start_udp_listener() {

    loop {
        if mottatt_udp {
            if min_wv_lavest_id > mottat_wv_lavest_id {
                //Oppdater egen WV
            }
        }
    }
}



fn start_tcp_listener() {
    loop {
        let prev_is_master = is_master;
        let is_master = min_id == wv_lavest_id;
        if is_master & !prev_is_master {
            //Koble fra tilkobling på master_connection
            
        }
        else if is_master {
            //Aksepter inkommende connections -> legg til i connection-array.
            //Send tasks mottatt fra task-kanal til riktig heis
            //Hvis ikke ACKA eller annet feil -> si fra til worldview
        } 
        else if !is_master & prev_is_master {
            //Koble fra alle slave-connections
            //koble til master, joinhandle er master_connection
        }
        else if !is_master {
            //Vent på å motta task
            //Mottat task, ACK den
            //Send mottat task på kanal til anvarlig for egen heis
        }
        
    }
}


fn start_process() {
    loop {
        let is_master = min_id == wv_lavest_id;

        if is_master {
            //Finn ut hvilken oppgaver som må gjøres
            //Deleger oppgaver til heiser
            //Send på kanal hvilken heis som skal gjøre hvilken task
        }
        else {
            //Vent på Task fra kanal ansvarlig for egen heis
            //Si fra når Task er gjort
        }
    }
}



