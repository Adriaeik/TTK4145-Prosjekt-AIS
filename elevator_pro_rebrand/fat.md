# Testskjema for Heisstyring

## Testmiljø
Test under tre ulike pakkatapsscenarier:
- **0% pakketap** – normal drift
- **30% pakketap** – moderat forstyrring
- **60% pakketap** – høg forstyrring

Resultat:
- ✅ Bestått
- ⚠️  Nesten bestått
- ❌ Ikkje bestått

---

## 1. Hall- og cab-bestillingar

| Test ID | Beskriving | 0% | 30% | 60% | Kommentar |
|--------|------------|-----|------|------|-----------|
| 1.1 | Hall-knapp trykkast. Ein heis skal komme. | ✅ | 🔲 | 🔲 | |
| 1.2 | Heis får motorstopp. Ein annan heis tek over. | ✅ | 🔲 | 🔲 | |
| 1.3 | Heis får obstruction. Ein annan heis tek over. | ✅  | 🔲 | 🔲 | Missforstått? |
| 1.4 | Alle heisar får cab-knappar trykka. | ✅ | 🔲 | 🔲 | |
| 1.5 | Cab-order 2 og 3. Motorstopp. Fullfører etter restart. | ✅ | 🔲 | 🔲 | |
| 1.6 | Obstruction i 2. Går til 3 etteråt. | ✅ | 🔲 | 🔲 | |
| 1.7 | Systemkræsj. Gjenopptar gamle cab-order etter restart. | ✅ | 🔲 | 🔲 | |

---

## 2. Feilhåndtering og deteksjon

| Test ID | Beskriving | 0% | 30% | 60% | Kommentar |
|--------|------------|-----|------|------|-----------|
| 1.8 | Motorstopp. Immobilitet detektert < 4s. | ⚠️ | 🔲 | 🔲 | -enkel fiks (truleg)|
| 1.9 | Obstruction. Immobilitet detektert < 9s. | ⚠️ | 🔲 | 🔲 | -enkel fiks (truleg) |
| 1.10 | Heis blir offline. Skal framleis ta ordre. | ✅ | 🔲 | 🔲 | |
| 1.11 | Heis får ordre, blir offline. Ny heis tek over. | ✅ | 🔲 | 🔲 | ca 10sek |
| 1.12 | Heis krasjar. Andre adopterer ordrene. | ✅ | 🔲 | 🔲 | |
| 1.13 | Heis går offline etter å ha fått hall-order. Både den og ein annan tek den. | ✅ | 🔲 | 🔲 | |
| 1.14 | Motor av/på. Nye order etterpå blir utført. | ✅ | 🔲 | 🔲 | |
| 1.15 | Nettverk av/på. Nye order og kommunikasjon reetableres. | ✅ | 🔲 | 🔲 | |

---

## 3. Effektivitet og fordeling

| Test ID | Beskriving | 0% | 30% | 60% | Kommentar |
|--------|------------|-----|------|------|-----------|
| 3.0 | id0:0, id1:0, id2:2. Hall 3 ned. id2 tek. | ✅ | 🔲 | 🔲 | Algoritme som vi fekk utdelt.. vi er nøgd |
| 3.1 | id0:0, id1:1, id2:2. Tre ordre fordelt korrekt. | ✅ | 🔲 | 🔲 | |
| 3.2 | id2 motorstopp. id1 tek over. | 🔲 | 🔲 | 🔲 | |
| 3.3 | id2 motorstopp + id1 mister nett. id0 tek over. | 🔲 | 🔲 | 🔲 | |
| 3.4 | id2 mister nett. id2 og id1 tek ordren. | 🔲 | 🔲 | 🔲 | |

---

## 4. Lys og knappelogikk

| Test ID | Beskriving | 0% | 30% | 60% | Kommentar |
|--------|------------|-----|------|------|-----------|
| 4.1 | Hall-knapp viser likt på alle PC-ar. | 🔲 | 🔲 | 🔲 | |
| 4.2 | Alle knappelys er synkroniserte. | 🔲 | 🔲 | 🔲 | |
| 4.3 | Cab-lys berre synleg for eigarheis. | 🔲 | 🔲 | 🔲 | |
| 4.4 | Knappelys tennast raskt etter trykk. | 🔲 | 🔲 | 🔲 | |
| 4.5 | Knappelys slår seg av etter utført ordre. | 🔲 | 🔲 | 🔲 | |

---

## 5. Dør og hindring

| Test ID | Beskriving | 0% | 30% | 60% | Kommentar |
|--------|------------|-----|------|------|-----------|
| 4.6 | Dørlys ikkje på under bevegelse. | ✅ | 🔲 | 🔲 | |
| 4.7 | Obstruction hindrar dør i å lukkast. | ✅ | 🔲 | 🔲 | |

---

## Kommentar
- Systemet skal testast under 0%, 30% og 60% pakketap.
- Resultata skal dokumenterast med vurdering og kommentarar for kvar test.

---

*Oppdatert: 2025-03-25*