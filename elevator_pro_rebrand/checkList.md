

# **Sjekkliste for Testing av Elevator Project**  
✅ = Bestått  
❌ = Feil funne  
🟡 = Delvis bestått / treng meir testing  

---

## **1. Hovudkrav**

### **1.1. Servicegaranti for knappelys**
- [ ] Når ei hall call-knapp blir trykt inn, lyser knappen og ein heis kjem til etasjen  
- [ ] Når ei cab call-knapp blir trykt inn, lyser knappen og **berre** den aktuelle heisen tek bestillinga  

### **1.2. Ingen samtaler (calls) skal gå tapt**
- **Nettverk og systemfeil:**
  - [ ] Simuler nettverksfeil: Ein heis mistar tilkopling – skal fortsatt handtere eksisterande bestillingar  
  - [ ] Simuler strømtap på ein heis – bestillingar blir ikkje gløymde etter restart  
  - [ ] Simuler programkrasj – systemet må kunne hente inn igjen bestillingane ved oppstart  
  - [ ] Simuler motorfeil – andre heisar tek over eksisterande hall calls  
  - [ ] Simuler at ein ny heis blir lagt til nettverket – han skal kunne hente bestillingar  

- **Kva skjer under nettverksfeil?**
  - [ ] Heis kan framleis ta nye cab calls og utføre eksisterande bestillingar  
  - [ ] Hall call-lys fungerer korrekt, men kan ha forsinka oppdatering  

### **1.3. Korrekt oppførsel av knappar og lys**
- [ ] Hall call-knappar på alle arbeidsstasjonar viser same status under normale forhold  
- [ ] Ved nettverkspaketttap, er det berre ei forsinking i oppdatering av lys  
- [ ] Cab call-lys er **ikkje** delt mellom heisar  
- [ ] Knappelys skrur seg på straks etter knappetrykk  
- [ ] Knappelys skrur seg av etter at bestillinga er gjennomført  

### **1.4. Dørkontroll**
- [ ] Dør-lyset skal ikkje vere på medan heisen beveger seg  
- [ ] Døra held seg open i **3 sekund** ved etasjestopp  
- [ ] Simuler dørhindring: Døra må ikkje lukke seg før hindringa forsvinn  
- [ ] Obstruksjonsbrytar kan aktiverast/deaktiverast når som helst utan feil  

### **1.5. Logisk og effektiv heisoppførsel**
- [ ] Heisen stoppar **berre** på relevante etasjar (ikkje alle for sikkerheit)  
- [ ] Heis som kjem til ein etasje med både opp og ned-bestillingar:  
  - [ ] Riktig retningskall blir prioritert  
  - [ ] Dersom ingen i heisen skal opp, men fleire skal ned, skal retningskall ryddast korrekt  

---

## **2. Sekundærkrav – Effektiv fordeling av oppdrag**
- [ ] Test at bestillingar blir fordelt mellom heisar for minimal ventetid  
- [ ] Simuler scenario med fleire samtidige bestillingar – raskaste heis bør bli tildelt oppdraget  

---

## **3. Tillatne antakingar**
- [ ] Systemet fungerer når minst éin heis ikkje er i feiltilstand  
- [ ] I eit system med fleire heisar, mistar ikkje ein isolert heis fleire funksjonar  
- [ ] Ingen nettverks-partisjonering (dvs. aldri fullstendig splitta nettverk)  

---

## **4. Test av uspesifisert oppførsel**
- [ ] Test om systemet startar i "single-elevator mode" om nettverket manglar  
- [ ] Test kva som skjer med hall call-knappar på ein isolert heis (valfri implementasjon)  
- [ ] Test kva "stopp-knappen" gjer dersom implementert  

---

## **5. Fleksibilitet i konfigurasjon**
- [ ] Test systemet med `n = 1, 2, 3` heisar  
- [ ] Test systemet med `m = 4` etasjar  
- [ ] Test om det er lett å legge til ein fjerde heis utan kodeendringar  
- [ ] Test om `--id <number>` flagget fungerer for å sette ID på ein heis  

---
