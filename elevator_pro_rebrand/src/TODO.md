Link til crates.io: https://crates.io/crates/elevatorpro

doc ligger der, direkte til doc: https://docs.rs/elevatorpro/latest/



- Lage funksjoner for update_wv i world_view_ch ✅
- Teste at det fortsatt funker (husk å teste etter hver hjelpefunksjon!!!!!)✅
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!✅
Ein god gitter pusher alltid ✅

- Legg til at master ACKer TCP meldinger, så slaven ikke sender ny før master har behandla den?
- Sync hos slaven så den passer på å ikke fjerne buttoncalls som ikke er sendt på TCP enda✅

- Når UDP melding blir for stor blir det feil i mottak. Hvis det fortsatt er problem når vi delegerer tasks, se på mulighet å  indikere i meldinga at den blir delt opp :)
- Prøv å sette buffer på mottak til mye større!! (legg til i config så det er lett å endre)✅

- Noen buttoncalls som forsvinner. mest sansynlig hos slaven før den blir sendt til master?✅

- Master bør ACKe buttoncalls (og tasks?) så vi ikke fjerner knapper om master dør før den fikk det med seg

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


- https://doc.rust-lang.org/rustdoc/write-documentation/what-to-include.html


- Det virker som pakketap ikkje er så farlig når du er slave, men drepandes for nettverket når du er master
- visst vi har hatt packetloss og blir fjerna av masteren så kjem vi først tilbake etter ein inside btn blir trykt?? wtf?? kan ha vert tilfeldig
- taper alle Calls når vi kobler oss tibake på nettverket etter packet loss

- Jævlig rar errorbugg som er vanskelig å finne eit mønster til..