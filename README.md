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


## Novità della 1.3.4

- **Selezione multipla con Ctrl**: tieni premuto Ctrl e clicca sugli utenti
  (nella lista membri o in un canale vocale) per selezionarne più di uno;
  col tasto destro su uno dei selezionati hai le azioni per tutti insieme.
  Se hai i permessi puoi spostarli in un altro canale vocale,
  disconnetterli, silenziarli o assordarli sul server, metterli in timeout,
  espellerli o bannarli con un colpo solo; in più menzionarli, aggiungerli
  agli amici, ignorarli o bloccarli. Chi non puoi toccare (per esempio un
  amministratore per il timeout) viene saltato. Un clic normale annulla la
  selezione.
- **Esempio dei secondi nel timeout**: tra gli esempi della durata ora c'è
  anche `60s`.

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
