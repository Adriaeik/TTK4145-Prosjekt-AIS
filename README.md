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
