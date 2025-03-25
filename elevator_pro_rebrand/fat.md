Reduser antallet ordertimers
Hall order lys når det allerede ligger en bestilling i den etasjen
SPM:
Hvor mye tid er ok fra lyset går på til noen tar den? (systemkrasj mens en heis er på vei dit. er det ok med 25-30 sek før en annen kommer dit?)
No orders are lost
Once the light aon a hall call button is turned on, an elevator should arrive at that floorx
1.1 Elevators are on differet floors. Hall button is pressed on some floor. If the light goes on, one elevator should then arrive at that floor.
1. 2 Elevators are on different floors. Hall button is pressed on some floor, where no elevator is present. Elevator heading for that floor gets motorstop. Another elevator should then go to that floor. 
1.3 Elevators are on different floors. Hall button is pressed on some floor, where no elevator is present. Elevator heading for that floor gets obstructed at different floor. Another elevator should then go to that floor. 
Similarly for a cab call (for telling the elevator what floor you want to exit at; front 4 buttons on the control panel), but only the elevator at that specific workspace should take the order
1.4 All elevators get all their cab-buttons pressed. They should all go to those respective floors.
1. 5 An elevator gets a cab call to floor 2 and 3 when in floor 0. It then gets motorstop. Itw should go to floor 2 then 3 when motorstop is done.
1.6 An elevator gets a cab call to floor 2 and 3 when in floor 0. It then gets obstructed in floor 2.  It should go to floor 3 when no longer obstructed is done.
1.7 The whole system crashes. It restarts. Elevator should go to all cab-orders it had before the system crashed.aq
This means handling network packet loss, losing network connection entirely, software that crashes, and losing power - both to the elevator motor and the machine that controls the elevator
For cab orders, handling loss of power/software crash implies that the orders are executed once service is restored
Already tested in 1.7
The time used to detect these failures should be reasonable, ie. on the order of magnitude of seconds (not minutes)
1.8 Send some orders to elevators, then trigger motorstop on one elevator. IMMOBILITY should be detected after less than 4 seconds.
1.9 Send some orders to the elevators, then trigger obstruction on one elevator. IMMOBILITY should be detected after less than 9 seconds. / obstruction in 3-4 sek
1.10 Send some orders to the elevators. Take one elevator offline. It should go to that floor itself within seconds.
1.11 Send some orders to the elevators. Take one offline after that elevator has started heading for a hall-order. One other elevator should then take that hall-order within seconds. 
1.12 Send some orders to the elevators. System crashes on an elevator that has orders. The other elevators should adopt those orders. 
If the elevator is disconnected from the network, it should still serve all the currently active orders (ie. whatever lights are showing)
It should also keep taking new cab calls, so that people can exit the elevator even if it is disconnected from the network
1.13 Assign hall order near one elevator, then disconnect from network. The elevator that was disconnected should take the order + another elevator should take the order. Take new cabcalls and see that the elevatorff executes these while being offline. Check lights. 
The elevator software should not require reinitialization (manual restart) after intermittent network or motor power loss
1.14 Send some orders, then turn off the motor. Then start the motor. Send some new orders and see that the elevator takes these. 
1.15 Send some orders, then turn off network. Take some orders, then turn on network. See that it communicates with the other elevators again by assigning new orders. 
Multiple elevators should be more efficient than one
The orders should be distributed across the elevators in a reasonable way
Ex: If all three elevators are idle and two of them are at the bottom floor, then a new order at the top floor should be handled by the closest elevator (ie. neither of the two at the bottom).q
Test:
id0: 0
id1: 0
id2: 2
id0 trykke 3 ned. 
=> id2 tar 3

id0: 0
id1: 1
id2: 2
id0 trykk 3 ned, id1 trykk 0 opp, id0 trykk 1 ned
=> id0 tar 0, id1 tar 1, id2 tar 3

id 0: 0
id1: 1
id2: 2
id0 trykk 3 ned, id2 får motorstopp => id1 tar 3
id0 trykk 3 ned, id2 får motorstopp, id1 mster nett => id0 tar 3
id0 trykk 3 ned, id2 mister nettverk => id2 tar den og id1 tar den. id0 blir værende. 
An individual elevator should behave sensibly and efficiently
No stopping at every floor "just to be safe"
3.1 Alle heiser i E0. HX D3. HX U2 mens heis i bevegelse. H0 stopper kun i 2 og 3.
The hall "call upward" and "call downward" buttons should behave differently
3.2 H0 E0, HX E1. HX D3 D2. H1 stopper i E3 så E2.
The lights and buttons should function as expected
The hall call buttons on all workspaces should let you summon an elevator
4.1 samme utgangspunkt for alle heisene, men bytt på hvilken pc som trykkes på.
Under normal circumstances, the lights on the hall buttons should show the same thing on all workspaces
4.2 Hver gang et knappelys lyser på en heis skal alle de andre også gjøre det. 
Under circumstances with high packet loss, at least one light must work as expected

The cab button lights should not be shared between elevators
4.3 Tykk på caborder på de forskjellige heisene og påse at kun lysene på den heisen lyser.
The cab and hall button lights should turn on as soon as is reasonable after the button has been pressed
Not ever turning on the button lights because "no guarantee is offered" is not a valid solution
You are allowed to expect the user to press the button again if it does not light up
The cab and hall button lights should turn off when the corresponding order has been serviced
4.5 gjør en bestilling til en etasje og påse at alle heiser skrur av lyset når orderen blir utført. Sjekk også for når en heis står i etasjen.
The "door open" lamp should be used as a substitute for an actual door, and as such should not be switched on while the elevator is moving
The duration for keeping the door open should be in the 1-5 second range
4.6 Følg med på at dørlyset ikke skrur seg på når heisen ikke står i en etasje.
The obstruction switch should substitute the door obstruction sensor inside the elevator
The door should not close while it is obstructed
4.7 Obstruer heisen når den er i bevegelse, neste gang den åpner døren vil den ikke lukkes før bryteren er av. 

