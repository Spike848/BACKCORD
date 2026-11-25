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


## Novità della 1.5.1

- **Aggiungi al gruppo**: nel menu di un amico (e della sua DM) c'è
  "Aggiungi al gruppo", con i tuoi gruppi in cui lui non c'è ancora e che non
  sono pieni. Se sei nella chiamata di un gruppo, quel gruppo compare per primo
  con la scritta "Nella chiamata" e l'amico aggiunto riceve la chiamata.
- **Ruoli con icona**: nel menu dei ruoli di un membro compaiono le immagini e
  le emoji dei ruoli, nei server potenziati che possono usarle.
- **Sottomenu lunghi**: un sottomenu più alto della finestra ora scorre.

## Cosa fa

- Server, canali, messaggi diretti, thread, forum, sondaggi, reazioni, emoji e GIF
- Chiamate vocali e video, condivisione schermo e visione delle dirette altrui
- Stati dei canali vocali, profili, ruoli, decorazioni e targhette
- Temi **BACKDOOR** (nero e rosso), **Discord**, **Vencord** e **Testcord**, con editor per creare i tuoi temi

### Funzioni aggiuntive

Attivabili da **Impostazioni → Funzioni aggiuntive**, ognuna con il suo interruttore
e le opzioni avanzate dall'ingranaggio:

| Funzione | Cosa fa |
| --- | --- |
| Ruoli cliccabili | Clic su un ruolo nel profilo per vederne i dettagli |
| No Push To Talk | Parla a rilevamento vocale anche dove il server impone il push-to-talk (spento di default) |
| Message Logger | Tiene i messaggi eliminati (in rosso) e la cronologia delle modifiche, volendo anche i propri; tasto destro → rimuovi dalla chat (spento di default) |
| Silent Typing | Nasconde "sta scrivendo…" |
| View Icons | Apre avatar, banner e icone in grande (fino a 1024 px, anche animati), con download |
| Stream Proof | Nasconde la finestra di BACKCORD dalle catture e dirette dello schermo |
| Emula piattaforme | Mostra accanto ai nomi da quali piattaforme le persone sono online (desktop, telefono, web, console, VR), nel colore del loro stato; sceglie la piattaforma su cui Discord ti vede, a tre livelli di emulazione |
| Anti-disconnessione | Rientra subito nel canale vocale da cui ti hanno disconnesso, se esiste ancora e puoi entrarci (spento di default) |
| Segui | Tasto destro su una persona → Segui: entri nei canali vocali in cui va, nei server in comune, finché non lo togli |
| Finto muto / finte cuffie mute | Dalla freccetta del microfono o delle cuffie: gli altri ti vedono mutato, ma parli e senti lo stesso |

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
