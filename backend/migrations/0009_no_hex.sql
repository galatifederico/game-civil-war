-- Le board esagonali non esistono piu': il client e' un gioco a caselle viste dall'alto.
-- Quelle gia' presenti (l'Alveare del mondo di prova) diventano quadrate: le coordinate restano
-- valide, cambia solo il numero di vicini (da 6 a 8). Idempotente come le altre.

UPDATE boards SET grid_kind = 'square' WHERE grid_kind = 'hex';
