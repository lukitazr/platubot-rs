# Platubot: gestión de torneos

Bot de Discord con PostgreSQL. Configura `DISCORD_TOKEN` y `DATABASE_URL` en el entorno o en `.env`, y ejecuta `cargo run`. Al arrancar aplica las migraciones automáticamente. Los comandos se registran en el servidor configurado en `src/main.rs`.

Los administradores con permiso **Gestionar servidor** pueden usar:

- `/torneo crear`: crea un torneo oficial o amistoso.
- `/torneo inscribir`: inscribe a un usuario.
- `/torneo programar`: añade un enfrentamiento entre participantes con su número de fecha.
- `/torneo capturas`: configura un canal de envío y otro de revisión para un torneo activo. Cada canal de envío pertenece a un único torneo.
- `/torneo resultado`: selecciona el torneo y un partido pendiente de las sugerencias; abre un formulario para introducir los goles de local y visitante, registra el marcador una sola vez y actualiza las estadísticas globales y del torneo.
- `/torneo wo`: identifica el partido por fecha y jugadores y registra el marcador por incomparecencia; el ausente debe ser el jugador derrotado y recibe un WO.
- `/torneo finalizar`: designa a un participante como campeón y suma el título correspondiente. No puede haber partidos pendientes.

Cualquier usuario puede usar:

- `/torneo fixture` (o `/fixture`): muestra una imagen por fecha con botones para navegar (◀ / ▶). Podés elegir la `fecha` inicial del fixture general. Con `jugador`, muestra todas sus fechas en una sola imagen, en dos columnas si la cantidad de fechas es par y en una si es impar. Dentro de cada fecha se aplica la misma regla a la cantidad de partidos.
- `/torneo tabla` (o `/tabla`): envía la clasificación del torneo como una imagen PNG, con puntos, partidos jugados, goles, victorias, empates, derrotas, WO y los últimos cinco resultados. Si hay más de 30 participantes, envía varias imágenes con los puestos en orden.
- `/torneo ver`: muestra la información completa del torneo con la clasificación y los partidos.
- `/user_info`: muestra las estadísticas globales de un usuario.

La clasificación otorga tres puntos por victoria y uno por empate, con diferencia de goles y goles a favor como desempates. La elección del campeón corresponde al administrador. Los torneos pertenecen al servidor en que se crean.

Las estadísticas se guardan como `BIGINT` no negativo de PostgreSQL, limitado a 2⁶³−1; los títulos están limitados a 2³²−1. Los datos anteriores de usuarios se conservan al aplicar la migración. Los resultados registrados no se pueden editar desde el bot.

Al escribir el argumento `torneo` de un comando, Discord sugiere hasta 25 torneos del servidor cuyo nombre o ID comienza por el texto introducido. Selecciona una sugerencia o escribe el ID completo; también puedes introducir un prefijo de nombre si identifica un único torneo. Si varios nombres coinciden, selecciona la sugerencia o usa el ID. La búsqueda por nombre no distingue mayúsculas de minúsculas.

En `/torneo programar`, `/torneo finalizar` y `/torneo wo`, selecciona primero el torneo: los campos de jugadores sugieren hasta 25 participantes inscritos, filtrados por el comienzo del nombre o ID. Selecciona una sugerencia o escribe el ID completo del participante. La inscripción sigue permitiendo seleccionar a cualquier usuario para añadirlo al torneo.

Las fechas se numeran dentro de cada torneo. Para registrar un resultado, selecciona primero el torneo y luego el partido pendiente; las sugerencias se filtran por jugador, fecha o ID y muestran hasta 25 partidos. Para registrar un WO, selecciona los dos jugadores y asigna a cada uno sus goles; el orden en que los elijas no altera la localía. Si ese duelo existe en otra fecha, el bot indica la fecha más cercana y su local y visitante. Si hay dos fechas a la misma distancia, muestra la anterior. Los partidos conservan un identificador interno independiente en la base de datos, que se muestra al seleccionar capturas.

`/torneo sortear` genera un calendario aleatorio de todos contra todos a partir de los inscritos, con una ronda por defecto (máximo 20). Cada fecha tiene hasta un partido por participante; con un número impar de participantes hay un descanso. Cada ronda par invierte las localías y el orden de las fechas de la anterior. El calendario limita a dos fechas seguidas como local o visitante; una fecha de descanso interrumpe la racha. Para igualar exactamente las localías y visitas, con un número par de participantes es necesario indicar un número par de rondas. El sorteo admite hasta 64 participantes y no modifica torneos que ya tienen partidos. Si no encuentra un calendario válido dentro del límite de búsqueda, puedes volver a intentarlo.

En el canal de envío, un participante puede adjuntar una o varias imágenes en un mensaje. El bot muestra un menú con sus partidos pendientes; puede seleccionar el partido correspondiente o pulsar **Ignorar envío**. Si hay más de 25 partidos, los botones permiten recorrer las páginas. Las imágenes se copian al canal de revisión. Allí, un administrador con permiso **Gestionar servidor** puede denegarlas o validar el resultado mediante un formulario con los goles del local y del visitante. La validación registra el resultado y actualiza las estadísticas una sola vez. El bot necesita acceso para leer el historial y enviar mensajes y archivos en ambos canales. Si falla el reenvío, el jugador puede volver a intentarlo con el mismo botón.
