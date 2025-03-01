- Lage funksjoner for update_wv i world_view_ch
- Teste at det fortsatt funker (husk å teste etter hver hjelpefunksjon!!!!!)
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
Ein god gitter pusher alltid

- Master sliter å lese TCP meldinger etter å ha stått litt (etter et par min hvertfal, finn ut hvor lenge). Finn ut akkuratt hva som får det til å skje. Kanskje watchdogen?
Prøv at master acker, slave sender ikke før den har fått ack
Prøv å sende tregere?
Hvis ikke, kanskje en annen tråd henger som gjør at tcp-en ikke klarer å oppdatere wv? 


- Delegere tasks (veldig enkelt. ingen prioritering ogsånt)
- Fjerne fullførte tasks
- Test for tap av tasks

- Offlinemode
- Backupmode

- Optimaliser Delegering av task (mer optimal fordeling av task)

- Fikse at elev server starter over ssh (se om vi kan beholde at den startes uten terminal)
- Har vi ein bug med TCP timeout? løyse det ??
- Teste med pakketap på nettverket

- Pynt. gjør ting fint så vi får bra karakter
- Dokumentasjon !! (flowchart?, kommentarer)
- Pynte utskrift av WV