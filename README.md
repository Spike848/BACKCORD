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


## Novità della 1.5.3

- **Dividi messaggi lunghi**: un messaggio oltre il limite di Discord (2.000
  caratteri, 4.000 con Nitro) parte in più messaggi, divisi a fine riga e tra
  le parole, senza spezzare blocchi di codice, menzioni e link. Nuova funzione
  aggiuntiva, attiva di default.
- **Chi sta scrivendo nella lista**: puntini animati accanto ai canali, alle DM e
  ai gruppi in cui qualcuno sta scrivendo.
- **Conteggio membri**: in cima alla lista membri "Membri — N · Online — M",
  anche nelle DM di gruppo.
- **Colore delle cartelle**: tasto destro su una cartella di server →
  "Colore cartella": tavolozza, colori di Discord e nome, salvati sul tuo
  account.
- **Tavolozza nel creatore temi**: clic sul quadratino di un colore per
  sceglierlo dalla tavolozza.
- **Esporta chat dall'intestazione**: bottone nella barra in alto di canali,
  DM e gruppi.
- **Gruppi**: il proprietario può rendere proprietario un altro membro o
  rimuoverlo dal gruppo, anche dal menu della chiamata.
- **Avviso amicizie**: ora anche per le richieste d'amicizia ricevute e per
  quelle accettate.
- **Correzioni**: la scheda degli inviti non si sovrappone più al resto; il
  cambio account non riporta più alla schermata di accesso; il tasto
  **Aggiorna** funziona anche per le copie installate in Programmi e riavvia
  BACKCORD in modo affidabile dopo l'aggiornamento.

## Cosa fa

- Server, canali, messaggi diretti, thread, forum, sondaggi, reazioni, emoji e GIF,
  con la scheda degli inviti sotto i messaggi (Entra in un clic) e l'anteprima
  dei messaggi linkati
- Chiamate vocali e video, condivisione schermo e visione delle dirette altrui
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
