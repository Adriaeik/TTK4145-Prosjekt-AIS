En del ting som må gjøres: 
- Sikre at deling av tasks skjer uten tap
- Rydde opp i nettverkkode: spesielt tcp
- Sette opp TCP watchdog (en struct som kan startes hver gang vi leser tcp) så vi kan oppdage når slaver dør
- Sikre at elev_container vi får fra slave stemmer sånn ca med elev_container vi trodde den slaven har. Om store avvik (da at den ikke har fått med seg task) send NACK, hvis ikke send ACK -> Slaven må også handle på om den får ACK eller NACK. Hvis slaven har markert en task som ferdig, husk også å fjerne tasken fra tasks. 
- Rydd opp i prosjekt-mappa. Tror worldview_test kan fjernes helt. Tror også tokio_ny kan fjernes, dobbeltsjekk om det er noe der som kan brukes senere. tokio må vente litt med, den har endel ti backup.