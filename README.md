# Platubot: gestión de torneos

Bot de Discord con PostgreSQL. Configura `DISCORD_TOKEN` y `DATABASE_URL` en el entorno o en `.env`, y ejecuta `cargo run`. Al arrancar aplica las migraciones automáticamente. Los comandos se registran en el servidor configurado en `src/main.rs`.

Los administradores con permiso **Gestionar servidor** pueden usar:

- `/torneo crear`: crea un torneo oficial o amistoso.
- `/torneo inscribir`: inscribe a un usuario.
- `/torneo programar`: añade un enfrentamiento entre participantes con su número de fecha.
- `/torneo resultado`: registra el marcador una sola vez y actualiza las estadísticas globales y del torneo.
- `/torneo wo`: registra un marcador por incomparecencia; el ausente debe ser el jugador derrotado y recibe un WO.
- `/torneo finalizar`: designa a un participante como campeón y suma el título correspondiente. No puede haber partidos pendientes.

`/torneo ver` muestra la clasificación y los partidos; `/user_info` muestra las estadísticas globales. La clasificación otorga tres puntos por victoria y uno por empate, con diferencia de goles y goles a favor como desempates. La elección del campeón corresponde al administrador. Los torneos pertenecen al servidor en que se crean.

Las estadísticas se guardan como `BIGINT` no negativo de PostgreSQL, limitado a 2⁶³−1; los títulos están limitados a 2³²−1. Los datos anteriores de usuarios se conservan al aplicar la migración. Los resultados registrados no se pueden editar desde el bot.
