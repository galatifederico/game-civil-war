# 08 — Mercato dinamico e negozi

Priorità: 8 (serve alla demo).

- `MarketEngine`: per oggetto prezzo = base × (domanda/offerta)^elasticità × (1 + interruzione logistica)
  × shock temporanei, limitato tra min e max. Domanda/offerta con media mobile, mercati locali per zona.
- Shock da notizie, eventi globali, danni strutturali.
- `ShopFramework`: edificio con proprietario (entità o fazione), catalogo, ricarico, prezzi fissati;
  le pedine comprano col proprio salario per soddisfare i bisogni; incasso al proprietario (→ tesoro di gilda).
