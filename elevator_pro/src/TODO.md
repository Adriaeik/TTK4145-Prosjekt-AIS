- Lage funksjoner for update_wv i world_view_ch ✅
- Teste at det fortsatt funker (husk å teste etter hver hjelpefunksjon!!!!!)✅
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!✅
Ein god gitter pusher alltid ✅

- Legg til at master ACKer TCP meldinger, så slaven ikke sender ny før master har behandla den?
- Sync hos slaven så den passer på å ikke fjerne buttoncalls som ikke er sendt på TCP enda✅

- Når UDP melding blir for stor blir det feil i mottak. Hvis det fortsatt er problem når vi delegerer tasks, se på mulighet å  indikere i meldinga at den blir delt opp :)
- Prøv å sette buffer på mottak til mye større!! (legg til i config så det er lett å endre)✅

- Noen buttoncalls som forsvinner. mest sansynlig hos slaven før den blir sendt til master?✅

- Delegere tasks (veldig enkelt. ingen prioritering ogsånt)
- Fjerne fullførte tasks
- Test for tap av tasks

- Sliter litt når den starter i 0te etasje?
- Sliter litt når den står lenge uten buttoncall?


- Offlinemode
- Backupmode

- Optimaliser Delegering av task (mer optimal fordeling av task)

- Fikse at elev server starter over ssh (se om vi kan beholde at den startes uten terminal)
- Har vi ein bug med TCP timeout? løyse det ??
- Teste med pakketap på nettverket

- Pynt. gjør ting fint så vi får bra karakter
- Dokumentasjon !! (flowchart?, kommentarer)
- Pynte utskrift av WV




### Tanker om taskallocating

- Bruk av costfunction. 
insp 
```go
func calculateCost(order elevio.ButtonEvent) uint {
	var cost = abs(order.Floor - elevio.GetFloor())
	if cost == 0 && ic.GetDirection() != elevio.MD_Stop {
		cost += 4
	}
	if cost > 0 && (ic.GetDirection() == elevio.MD_Down || ic.GetDirection() == elevio.MD_Up) {
		cost += 3
	}
	if cost != 0 && ic.GetDirection() == elevio.MD_Stop {
		cost++
	}
	return cost
}
```