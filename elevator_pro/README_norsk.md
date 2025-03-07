# Elevator Network Control System

## Introduksjon
Dette prosjektet er en implementasjon av et distribuert heisstyringssystem utviklet i Rust som en del av TTK4145 – Sanntidsprogrammering ved NTNU. Systemet skal håndtere flere heiser i parallell, med fokus på robusthet, feilhåndtering og optimal oppgavefordeling.

## Status for prosjektet
Prosjektet er ikke ferdigstilt, men vi har implementert en betydelig del av systemets funksjonalitet. Nedenfor er en oppsummering av hva som er gjort, hva vi vil forbedre, og hva som gjenstår å implementere.

### Implementerte funksjoner
✅ **Distribuert heisnettverk**: Heisene kommuniserer via TCP og UDP for å oppdatere tilstand og håndtere oppgaver.
✅ **Master-Slave håndtering**: En heis velges som master og koordinerer oppgaver. Hvis en heis feiler, kan en annen ta over.
✅ **Grunnleggende oppgavefordeling**: Heisene får tildelt bestillinger, men det er fortsatt forbedringspotensiale i fordelingen.
✅ **Håndtering av nettverkskommunikasjon**: Pakker sendes mellom master og slaver, og worldview oppdateres kontinuerlig.

### Forbedringsområder
🔄 **Forbedret oppgavefordeling**: Cost-funksjonen for oppgavefordeling må optimaliseres for å sikre raskere og mer effektive heisbevegelser.
🔄 **Bedre feilhåndtering**: Dersom master-heisen dør mens en TCP-melding sendes, må en ny master kunne motta denne informasjonen.
🔄 **Håndtering av lys i heisen**: Foreløpig er knappelysene ikke implementert, og dette må løses for å møte kravene.
🔄 **Lokal backup for master/slave**: Hver enhet bør ha en inaktiv klone av programmets tilstand som kan ta over ved krasj eller manuelt avsluttet program (Ctrl+C).

### Gjenstående oppgaver
🔜 **Implementering av lokal backup for hver heis**
🔜 **Sikre at TCP-meldinger sendes til ny master dersom den opprinnelige dør**
🔜 **Fullføre lysstyring for heisen**
🔜 **Optimalisere oppgavefordeling med en forbedret cost-funksjon**

## Hvordan vi svarer på hovedoppgaven
Prosjektets mål er å skape et robust system hvor:
- Ingen bestillinger går tapt, selv ved nettverksfeil eller programkrasj.
- Systemet håndterer flere heiser parallelt på en effektiv måte.
- Heisene reagerer riktig på brukerinput og utfører oppgaver raskt og pålitelig.
- Systemet kan tåle feil og automatisk gjenopprette funksjonalitet.

Vi har lagt et godt grunnlag for systemet, men det er fortsatt arbeid som gjenstår før vi oppfyller alle hovedkravene i prosjektspesifikasjonen.

## Hvordan kjøre koden
For å kjøre systemet, følg disse stegene:
1. **Installer Rust** dersom du ikke allerede har det.
2. **Klon repoet**: `git clone https://github.com/Adriaeik/TTK4145-Prosjekt-AIS`
3. **Bygg prosjektet**: `cargo build`
4. **Kjør en heis-instans**: `cargo run`
5. **Start flere instanser** for å simulere flere heiser.

## Videre arbeid
Prosjektet er i aktiv utvikling, og vi ønsker tilbakemeldinger fra medstudenter og veiledere på hvordan vi kan forbedre systemet. Har du forslag eller ser potensielle feil? Gi oss beskjed!

---
_Takk for at du vurderer vårt arbeid! Vi ser frem til videre utvikling og optimalisering av systemet._

