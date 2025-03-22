#!/usr/bin/env python3
import os
from fpdf import FPDF

# PDF-klasse med hovud- og bunntekst
class PDF(FPDF):
    def header(self):
        self.set_font("Arial", "B", 12)
        self.cell(0, 10, "Innhald frå Rust-filer", ln=True, align="C")

    def footer(self):
        self.set_y(-15)
        self.set_font("Arial", "I", 8)
        self.cell(0, 10, "Side " + str(self.page_no()), align="C")

def rens_tekst(tekst):
    """Fjern teikn som ikkje kan kodast med latin-1"""
    return tekst.encode("latin-1", errors="ignore").decode("latin-1")

def main():
    # Be brukaren oppgi mappestien
    mappesti = "elevator_pro_rebrand"

    # Opprett PDF-objektet
    pdf = PDF()
    pdf.set_auto_page_break(auto=True, margin=15)
    pdf.set_font("Arial", "", 10)

    # Gå gjennom mappa rekursivt
    for rot, mapper, filer in os.walk(mappesti):
        for fil in filer:
            if fil.endswith(".rs"):
                filsti = os.path.join(rot, fil)
                pdf.add_page()
                # Skriv ut filbanen som overskrift
                pdf.cell(0, 10, rens_tekst(f"Fil: {filsti}"), ln=True)
                try:
                    with open(filsti, "r", encoding="utf-8") as f:
                        for linje in f:
                            pdf.multi_cell(0, 5, rens_tekst(linje.rstrip()))
                except Exception as e:
                    pdf.multi_cell(0, 5, f"Feil ved lesing av fila: {e}")

    # Lagre PDF-en
    utfil = "rust_filer.pdf"
    pdf.output(utfil)
    print(f"PDF er generert: {utfil}")

if __name__ == "__main__":
    main()

"""
Skjønner! Dere vil forbedre TCP-kommunikasjonen slik at:

Master kvitterer mottatte meldinger → Når en slave sender en melding til master via TCP, skal master sende tilbake en kopi av det den mottok for å bekrefte at meldingen gikk gjennom.
Resend ved master-fall → Hvis master dør mens en slave sender en melding, må slaven automatisk sende meldingen på nytt til den nye masteren.
Dette gir null pakketap, selv om en master går ned! 💡

Hvordan forbedre TCP-sikkerheten?
🔹 Steg 1: Master sender kvittering
I tcp_network.rs, når master mottar en melding fra en slave, kan vi gjøre følgende:

Etter at master mottar data via read_from_stream(), sender den samme data tilbake til slaven.
Slaven sjekker at mottatt data stemmer overens med det den sendte.
🔹 Steg 2: Slave venter på kvittering
I slaven:

Etter å ha sendt en melding, venter den på kvittering fra master.
Hvis kvitteringen ikke matcher eller ikke kommer innen en tidsgrense, prøver slaven på nytt.
🔹 Steg 3: Resend ved master-fall

Slaven lagrer meldinger i en "resend buffer".
Hvis den mister tilkoblingen til master, kobler den seg til en ny master.
Alle meldinger i "resend buffer" sendes til den nye masteren.
Kodeendringer
🔹 Endring i master (tcp_network.rs)

I read_from_stream(), legg til at master sender tilbake samme data:

rust
Kopier
async fn read_from_stream(stream: &mut TcpStream, chs: local_network::LocalChannels) -> Option<Vec<u8>> {
    let mut buf = vec![0; 1024];
    
    match stream.read(&mut buf).await {
        Ok(len) if len > 0 => {
            let received_data = buf[..len].to_vec();

            // Send kvittering tilbake til slaven
            if let Err(e) = stream.write_all(&received_data).await {
                utils::print_err(format!("Feil ved sending av kvittering: {}", e));
            }

            Some(received_data)
        }
        Ok(_) => None,
        Err(e) => {
            utils::print_err(format!("Feil ved lesing av TCP: {}", e));
            None
        }
    }
}
🔹 Endring i slave (tcp_network.rs)

I send_tcp_message(), legg til at slaven venter på kvittering:

rust
Kopier
async fn send_tcp_message(
    chs: local_network::LocalChannels,
    stream: &mut TcpStream,
    msg: Vec<u8>
) -> bool {
    // Send meldingen til master
    if let Err(e) = stream.write_all(&msg).await {
        utils::print_err(format!("Feil ved sending av TCP-melding: {}", e));
        return false;
    }

    // Vent på kvittering
    let mut response = vec![0; msg.len()];
    match stream.read_exact(&mut response).await {
        Ok(_) if response == msg => {
            utils::print_ok("Melding bekreftet av master.".to_string());
            true
        }
        _ => {
            utils::print_warn("Master svarte ikke riktig – sender på nytt!".to_string());
            false
        }
    }
}
🔹 Retry-mekanisme hvis master dør

Hvis master dør, må vi sende meldingen på nytt til neste master:

rust
Kopier
async fn resend_to_new_master(chs: local_network::LocalChannels, msg: Vec<u8>) {
    while let Some(mut new_master) = connect_to_master(chs.clone()).await {
        if send_tcp_message(chs.clone(), &mut new_master, msg.clone()).await {
            break; // Slutt hvis meldingen ble sendt og bekreftet
        }
        utils::print_warn("Ny master svarte ikke, prøver igjen...");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
Oppsummert
✅ Master sender tilbake det den mottok, som kvittering.
✅ Slaven sjekker kvitteringen – sender på nytt hvis noe mangler.
✅ Hvis master dør, lagres meldinger og sendes til ny master.

Dette vil sikre null pakketap og stabil kommunikasjon! 🚀
"""