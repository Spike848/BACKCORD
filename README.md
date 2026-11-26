<p align="center">
  <img src="docs/images/backcord-readme.png" width="320" alt="BACKCORD">
</p>

<h1 align="center">BACKCORD</h1>

<p align="center">
  <b>Un client Discord nativo, scritto in Rust, senza browser dentro.</b><br>
  Leggero, veloce, con voce, video, dirette e funzioni aggiuntive integrate.
</p>

<p align="center">
  <a href="https://github.com/Spike848/BACKCORD/releases/latest"><b>⬇ Scarica per Windows</b></a>
  &nbsp;·&nbsp;
  <a href="https://discord.gg/backdoor666"><b>💬 Server Discord di BACKCORD</b></a>
</p>

---

## Download e installazione (Windows 10/11)

1. Vai su [**Releases**](https://github.com/Spike848/BACKCORD/releases/latest) e scarica
   `BACKCORD-windows-x64.zip`.
2. Estrai lo zip in una cartella a tua scelta.
3. Avvia `BACKCORD.exe`.

Il programma non è firmato digitalmente, quindi al primo avvio Windows
SmartScreen potrebbe mostrare "Windows ha protetto il PC": clicca
**Ulteriori informazioni → Esegui comunque**.

Al primo avvio accedi con il tuo account Discord. Impostazioni, temi e cache
vengono salvati in `%LOCALAPPDATA%\Spike848\BACKCORD\data`. Il token
dell'account resta nel Gestore credenziali di Windows e la cache su disco è
cifrata.


## Novità della 1.5.9

- **Blur della stream**: in chiamata, vicino al tasto per uscire, un pulsante
  ben visibile sfoca tutta la tua stream per nascondere dati sensibili. Si
  accende e si spegne senza fermare la stream; parte spento.
- **Schermo intero pulito**: a schermo intero il pannello vocale non si
  sovrappone più alla stream; restano solo blur, esci e riduci.
- **Limite di qualità ricevuta**: accanto al tasto della fotocamera scegli la
  risoluzione (da 4K a 144p) e i frame al secondo (60, 30, 15) massimi che
  ricevi da fotocamere e stream, per risparmiare rete e CPU.
- **Selezione dei messaggi**: trascina un riquadro sui messaggi (o Ctrl+clic);
  col tasto destro cancelli solo i tuoi oppure tutti, se hai il permesso.
- **Selezione dei server**: Ctrl+clic o un riquadro sulla lista dei server; col
  tasto destro li segni come letti, li silenzi o li riattivi tutti insieme.
- **Selezioni più affidabili**: trascinare non lascia più un utente "a
  mezz'aria", e un clic in un altro punto dello schermo annulla qualsiasi
  selezione.
- **Persone in vocale sulla lista server**: la scheda che appare passando sopra
  un server mostra chi c'è davvero, anche nei canali silenziati, e non tiene
  più chi è passato a un altro server.
- **GIF e sticker in un solo pulsante** nella barra di scrittura.
- **Sfocatura del tema ovunque**: selettore emoji, GIF e sticker, pannello di
  formattazione, e nella modifica profilo i selettori di tag server, targhetta,
  effetto, cornice e badge.
- **Voce e video**: modalità d'ingresso, modalità studio, riduzione del rumore
  e dell'eco ora stanno sotto il microfono, accanto agli altoparlanti.
- **Stream senza "could not start the stream worker"**: se il processo
  separato della stream non può partire, la stream gira dentro BACKCORD invece
  di chiudersi subito.
- Corretti: le icone dei tag server in vocale che non si caricavano finché non
  aprivi il profilo, e il testo di "Crea sondaggio" che usciva dai bordi.

## Cosa fa

- Server, canali, messaggi diretti, thread, forum, sondaggi, reazioni, emoji e GIF,
  con la scheda degli inviti sotto i messaggi (Entra in un clic) e l'anteprima
  dei messaggi linkati
- Chiamate vocali e video, condivisione schermo e visione delle dirette altrui,
  con limitatore, compressore, passa-alto e gate sulle voci degli altri
- Messaggi vocali: registri, riascolti e invii dalla chat
- Stati dei canali vocali, profili, ruoli, decorazioni e targhette
- **Esporta chat**: tasto destro su un canale, una DM o un gruppo → **Esporta chat…**,
  in testo (TXT), pagina web (HTML) o JSON; tutta la chat, gli ultimi N messaggi o da
  una data, con gli allegati come link o senza. Mostra quanti messaggi ha scaricato,
  si può fermare e salva il file in Download (o dove scegli)
- Temi **BACKDOOR** (nero e rosso), **Discord**, **Vencord** e **Testcord**, con editor per creare i tuoi temi

### Funzioni aggiuntive

Attivabili da **Impostazioni → Funzioni aggiuntive**, ognuna con il suo interruttore
e le opzioni avanzate dall'ingranaggio:

| Funzione | Cosa fa |
| --- | --- |
| Ruoli cliccabili | Clic su un ruolo nel profilo per vederne i dettagli |
| Visualizzatore permessi | Tasto destro su un membro → **Permessi**: i suoi permessi effettivi nel server o nel canale aperto (con le eccezioni del canale), ognuno con spunta verde o croce rossa e il ruolo o l'eccezione che lo concede o lo toglie; proprietario e Amministratore hanno tutto, il timeout lascia solo vedere e leggere. **I miei permessi** nel menu di un server, canale o categoria; **Permessi** nella scheda di un ruolo per ciò che dà il ruolo |
| No Push To Talk | Parla a rilevamento vocale anche dove il server impone il push-to-talk (spento di default) |
| Dividi messaggi lunghi | Un messaggio più lungo di quanto Discord consente (2.000 caratteri, 4.000 con Nitro) parte in più messaggi, divisi a fine riga e tra le parole, senza spezzare blocchi di codice, menzioni e link |
| Message Logger | Tiene i messaggi eliminati (in rosso) e la cronologia delle modifiche, volendo anche i propri; tasto destro → rimuovi dalla chat (spento di default) |
| Silent Typing | Nasconde "sta scrivendo…" |
| View Icons | Apre avatar, banner e icone in grande (fino a 1024 px, anche animati), con download |
| Stream Proof | Nasconde la finestra di BACKCORD dalle catture e dirette dello schermo |
| Emula piattaforme | Mostra accanto ai nomi da quali piattaforme le persone sono online (desktop, telefono, web, console, VR), nel colore del loro stato; sceglie la piattaforma su cui Discord ti vede, a tre livelli di emulazione |
| Anti-disconnessione | Rientra subito nel canale vocale da cui ti hanno disconnesso, se esiste ancora e puoi entrarci (spento di default) |
| Segui | Tasto destro su una persona → Segui: entri nei canali vocali in cui va, nei server in comune, finché non lo togli |
| Anteprime dei link a messaggi | Sotto un messaggio con un link a un messaggio di Discord (anche DM, ptb e canary) mostra una scheda con autore, canale, testo e prima immagine; clic per andarci. Al massimo 2 per messaggio |
| Avviso amicizie | Ti avvisa (notifica e scheda **Avvisi amicizie** nella posta in arrivo) quando qualcuno ti invia una richiesta d'amicizia o accetta la tua, quando un amico ti rimuove o ti blocca, una richiesta di amicizia viene rifiutata o ritirata, vieni rimosso da un gruppo o cacciato/bannato da un server (o il server viene eliminato); all'avvio segnala anche cosa è cambiato mentre BACKCORD era chiuso. Non segnala ciò che fai tu. Ogni tipo di avviso si attiva a parte |
| Finto muto / finte cuffie mute | Dalla freccetta del microfono o delle cuffie: gli altri ti vedono mutato, ma parli e senti lo stesso |
| Nitro finto | Senza Nitro puoi scegliere nel selettore le emoji di tutti i server (anche animate) e gli sticker degli altri server: quelli che non potresti usare partono come link alla loro immagine, mascherati come `[nome](https://…)` o nudi, con dimensione delle emoji a scelta. I link di emoji ricevuti si vedono come emoji nel testo. Nessun trucco sulla qualità dello streaming (spento di default) |
| Auto-smuta | Se un server ti silenzia o ti toglie l'audio in un canale vocale dove puoi silenziare e togliere l'audio ai membri, lo annulla subito; si ferma dopo 5 volte in un minuto (spento di default) |
| Anti-cambio nickname | Se qualcun altro ti cambia o toglie il nickname in un server dove puoi cambiarlo tu, lo rimette com'era; vale anche per un cambio fatto da un altro tuo dispositivo (spento di default) |
| Anti chiamata di gruppo | Le chiamate nei gruppi DM non squillano; quando ne parte una esci dalla chiamata e dal gruppo, senza il messaggio che sei uscito. Per chiamare tu un gruppo va spenta (spento di default) |
| Auto-eliminazione | Cancella i messaggi che invii dopo il tempo scelto dall'ingranaggio, da 30 secondi a 7 giorni (5 minuti di default); quelli scaduti a BACKCORD chiuso vanno alla riapertura (spento di default) |
| Badge spoofer | Mostra sul tuo profilo (scheda, profilo completo, scheda del tuo account) i badge che scegli come se li avessi: Staff, Partner, Alunni programmi moderatori, Eventi HypeSquad, casata HypeSquad (Bravery, Brilliance, Balance), Cacciatore di bug (normale e d'oro), Sviluppatore attivo, Sviluppatore di bot verificato della prima ora, Sostenitore della prima ora, Missione completata, Nitro con i livelli di anzianità (da Bronzo a Opale) e Booster del server (da 1 mese a 2 anni), nell'ordine di Discord e con le sue immagini ufficiali; volendo accanto ai tuoi badge reali. Solo estetico e locale: li vedi solo tu su questo PC, a Discord non viene inviato nulla (spento di default) |

## Avvertenze

BACKCORD **non è affiliato, approvato o supportato da Discord**. È un client
non ufficiale: i Termini di Servizio di Discord non consentono i client di
terze parti, quindi usarlo comporta il rischio di provvedimenti sull'account.
Lo usi a tuo rischio.

## Compilare dal sorgente

Serve Linux (o WSL su Windows) con [Nix](https://nixos.org/download/) e i flake attivi.

```sh
git clone https://github.com/Spike848/BACKCORD.git
cd BACKCORD

# Linux
nix develop path:./nix --command cargo build --release

# Windows (cross-compilazione da Linux/WSL)
nix develop path:./nix#windows --command bash -c '
  source scripts/release-env x86_64-pc-windows-msvc &&
  export BACKCORD_RELEASE=1 &&
  cargo build --locked --release --target x86_64-pc-windows-msvc \
    --no-default-features --features desktop,discord'
# → target/x86_64-pc-windows-msvc/release/BACKCORD.exe
```

Documentazione tecnica in [`docs/`](docs/). Per proporre modifiche leggi
[`CONTRIBUTING.md`](CONTRIBUTING.md) o passa sul
[server Discord di BACKCORD](https://discord.gg/backdoor666).
