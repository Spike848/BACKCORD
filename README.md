<p align="center">
  <img src="packaging/backcord-logo.png" width="128" alt="">
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
vengono salvati in `%LOCALAPPDATA%\Spike848\BACKCORD\data` (dalla 1.2; la
cartella delle versioni precedenti viene spostata lì da sola). Il token
dell'account resta nel Gestore credenziali di Windows e la cache su disco è
cifrata.


## Novità della 1.2.3

- **Spostamenti in vocale corretti**: quando un bot (per esempio quello di un
  canale "crea stanza") ti sposta in un altro canale, la chiamata ti segue
  invece di disconnettersi con "Vocale disconnessa".

## Novità della 1.2.2

- **Cancella tutto**: tasto destro su un DM o su un server → *Cancella
  tutto* cancella, dopo una conferma, tutti i messaggi che hai mandato lì
  (in un server, in tutti i canali), uno alla volta con il ritardo minimo
  che Discord accetta. L'avviso in alto mostra quanti ne ha cancellati; dal
  menu si può fermare.

## Novità della 1.2.1

- **Più dirette insieme** (fino a 4), come su Discord
- Le dirette hanno un **riquadro separato** da quello della persona (la cam
  resta in vista), con anteprima sfocata, **Guarda diretta** e l'occhio per
  guardarne più di una
- Sulla diretta guardata: la **qualità scelta** da chi trasmette
  (es. `1080P 60FPS`) e LIVE, l'altoparlante per silenziarla e il menu **…**
  (Smetti di guardare, Silenzia, volume)
- I **temi personalizzati** di nuovo tra i temi in Aspetto

## Novità della 1.2

- **Sicurezza**: nuova sezione con proxy (SOCKS5/HTTP/HTTPS) e DNS cifrato
  (Cloudflare, Google, Quad9 o un server tuo)
- **Riduzione dell'eco** regolabile (Disattivata / Normale / Forte)
- **Dirette più fluide** a 60 fps, con meno artefatti, e cattura corretta dei
  giochi a schermo intero con lo schermo scalato
- Restare in chiamata quando un bot ti **sposta di canale**
- Si illumina chi fa partire un suono della **soundboard**; menu della
  soundboard con i nomi interi
- **Banner animati** dei server
- **Message Logger**: opzione per registrare anche i propri messaggi
- **Sicurezza rafforzata**: cookie cifrati, cartella e credenziali proprie di
  BACKCORD, import dei temi controllato, link aperti nel
  browser solo se sono indirizzi web
- Dicitura "Chiaro" rinominata in "Minima"

## Cosa fa

- Server, canali, messaggi diretti, thread, forum, sondaggi, reazioni, emoji e GIF
- Chiamate vocali e video, condivisione schermo e visione delle dirette altrui
- Stati dei canali vocali, profili, ruoli, decorazioni e targhette
- Tema **BACKDOOR** (nero e rosso), con editor per creare i tuoi temi

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

## Crediti e licenza

BACKCORD è distribuito sotto licenza
[GNU GPL v3.0](LICENSE): puoi usarlo, studiarlo, modificarlo e ridistribuirlo,
a patto che anche le versioni modificate restino libere con il loro sorgente.
