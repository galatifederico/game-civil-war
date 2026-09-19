# Esporre il gioco su internet (Cloudflare Tunnel)

Il server gira sul PC di casa; il tunnel di Cloudflare lo rende raggiungibile dagli amici senza aprire porte sul router né mostrare l'IP di casa. **Questa procedura non è stata provata da chi l'ha scritta** (serve un account Cloudflare e un dominio): quello che è stato verificato è che l'immagine del backend si costruisce e parte, che `docker compose config` accetta i profili e che il backend rispetta `TRUST_PROXY`. Il resto va provato la prima volta.

## Cosa serve

- Un account Cloudflare (gratuito) con un dominio gestito lì.
- Docker con Compose (già usato per Postgres).

## Passi

1. **Crea il tunnel.** Dashboard Cloudflare → Zero Trust → Networks → Tunnels → *Create a tunnel* → Cloudflared. Copia il **token** che mostra.
2. **Aggiungi un "public hostname"** al tunnel, per esempio `gioco.tuodominio.it`, con servizio `HTTP` e URL `backend:8090` (il nome del servizio nel compose).
3. **Configura `deploy/.env`** (il file non finisce su git):

   ```
   JWT_SECRET=<una stringa lunga e casuale, per esempio: openssl rand -hex 32>
   TRUST_PROXY=1
   TUNNEL_TOKEN=<il token del passo 1>
   ```

   `JWT_SECRET` non deve essere quello di sviluppo: chi lo conosce può fabbricarsi un accesso come qualunque giocatore. `TRUST_PROXY=1` fa contare i limiti di tentativi di accesso per indirizzo reale del client (header `CF-Connecting-IP`) invece che per quello del tunnel; **non attivarlo** se il backend è raggiungibile anche senza il tunnel, perché l'header si potrebbe falsificare.
4. **Avvia tutto:**

   ```
   docker compose -f deploy/docker-compose.yml --profile tunnel up -d --build
   ```

   Il profilo `tunnel` avvia Postgres, Redis, il backend (costruito da `backend/Dockerfile`) e `cloudflared`. Senza il tunnel, `--profile app` avvia solo il backend.
5. **Client Unity:** nel campo **Server** della schermata di accesso scrivi `https://gioco.tuodominio.it`. Il client passa da solo a `wss://` per il WebSocket quando l'indirizzo comincia con `https`. Il pannello admin è su `https://gioco.tuodominio.it/admin/`.

## Sicurezza: cosa c'è e cosa no

- Postgres e Redis sono pubblicati solo su `127.0.0.1`: il tunnel espone il backend e nient'altro. Se il compose è già in esecuzione, la modifica si applica al prossimo `make -C backend up` (ricrea i container, i dati restano nei volumi).
- Le password sono salvate con bcrypt; login e registrazione sono limitati a 20 tentativi al minuto per indirizzo. Ogni connessione di gioco è limitata a 15 comandi al secondo (burst 30) e viene chiusa se continua a sforare.
- **Non c'è ancora**: verifica dell'email, recupero password, scadenza/revoca dei token prima dei 30 giorni, log di audit delle azioni admin. Il pannello admin è protetto solo dall'account proprietario del mondo. Per giocare con pochi amici fidati va bene; per un pubblico aperto no.
- Cloudflare vede il traffico in chiaro (termina il TLS): va bene per un gioco tra amici, ma va saputo.

## Aggiornare

```
git pull
docker compose -f deploy/docker-compose.yml --profile tunnel up -d --build backend
```

Le migrazioni si applicano da sole all'avvio del backend (sono idempotenti). I giocatori connessi vengono scollegati e il client si riconnette da solo.
