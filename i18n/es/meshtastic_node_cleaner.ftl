## Aplicación

app-name = Meshtastic Node Cleaner
app-tagline = Limpia nodos de la nodedb de tu dispositivo
about = Elimina nodos antiguos o parcialmente conocidos de la nodedb de un dispositivo Meshtastic.

## Ayuda de la línea de órdenes

help-usage = Uso:
heading-connection = Conexión
heading-selection = Selección (por defecto: --unknown)
heading-output = Salida
heading-action = Acción
help-serial = Conectar por serie. DEVICE (p. ej. /dev/ttyUSB0) se detecta automáticamente si se omite. Nunca recurre a TCP
help-tcp = Conectar por TCP. HOST puede ser host, host:puerto o http://host:puerto; por defecto localhost:4403. Nunca recurre a serie
help-unknown = Seleccionar nodos parcialmente conocidos (sin NodeInfo completo). Es lo predeterminado si no se indica otra selección
help-older-than = Seleccionar nodos que no se oyen desde hace más de DURATION (p. ej. 30m, 24h, 7d, 2w). Junto con --unknown, deben cumplirse ambas
help-include-never = Con --older-than, seleccionar también los nodos sin hora de última escucha
help-remove = Eliminar exactamente estos nodos (p. ej. !aabbccdd, 0xaabbccdd o un número decimal)
help-sort-by = Columna por la que ordenar la tabla
help-reverse = Invertir el orden (fecha: los más antiguos primero; texto: Z→A)
help-verbose = Aumentar el detalle (-v datos de conexión, -vv depuración)
help-dry-run = Mostrar qué se eliminaría sin cambiar nada
help-yes = Eliminar sin pedir confirmación
help-set-time = Poner el reloj del dispositivo en la hora del sistema antes de leer la nodedb
help-help = Mostrar la ayuda
help-version = Mostrar la versión
help-examples =
    Ejemplos:
      meshtastic-node-cleaner                     Elimina los nodos parcialmente conocidos, detectando el dispositivo
      meshtastic-node-cleaner -n                  Solo muestra qué se eliminaría
      meshtastic-node-cleaner -s /dev/ttyUSB0     Usa un dispositivo serie concreto
      meshtastic-node-cleaner -t 192.168.1.10     Conecta por TCP
      meshtastic-node-cleaner -o 30d              Elimina los nodos que no se oyen desde hace 30 días
      meshtastic-node-cleaner -u -o 7d            Elimina los parcialmente conocidos que no se oyen desde hace 7 días
      meshtastic-node-cleaner -r '!aabbccdd'      Elimina un nodo concreto

    Los favoritos y los nodos ignorados nunca se eliminan.

## Errores de argumentos

error-duration = duración no válida '{ $value }'; usa p. ej. 30m, 24h, 7d o 2w
error-host = host no válido '{ $value }'; usa p. ej. 192.168.1.10 o 192.168.1.10:4403
error-node-id = identificador de nodo no válido '{ $value }'; usa p. ej. !aabbccdd

## Conexión

connecting-serial = Conectando por serie a { $device }…
connecting-tcp = Conectando por TCP a { $address }…
serial-multiple = Hay varios puertos serie ({ $ports }); se usa { $port }. Usa --serial DEVICE para elegir otro.
serial-none-fallback = No se ha detectado ningún dispositivo serie. Probando TCP en localhost…
error = Error:
error-connect = no se pudo conectar con el dispositivo
error-no-serial = no se ha encontrado ningún dispositivo serie; conecta uno o usa --tcp
error-config-timeout = se agotó el tiempo esperando la nodedb del dispositivo
error-connection-closed = el dispositivo cerró la conexión
error-no-my-info = el dispositivo no informó de su propio número de nodo
error-rejected = el dispositivo rechazó la petición ({ $reason })
error-set-time = No se pudo poner la hora del dispositivo:
time-set = Reloj del dispositivo puesto en la hora del sistema.
time-set-unconfirmed = Hora enviada, pero el dispositivo no lo confirmó.

## Tabla de nodos

nodedb-summary = { $count ->
    [one] El dispositivo tiene { $count } nodo en su nodedb.
   *[other] El dispositivo tiene { $count } nodos en su nodedb.
}
nodedb-empty = El dispositivo no tiene otros nodos en su nodedb.
to-delete = { $count ->
    [0] Ningún nodo que eliminar
    [one] { $count } nodo que eliminar
   *[other] { $count } nodos que eliminar
}
criteria-unknown = nodos parcialmente conocidos
criteria-older = sin oírse desde hace más de { $age }
criteria-unknown-older = parcialmente conocidos y sin oírse desde hace más de { $age }
criteria-include-never = (incluidos los nunca oídos)
criteria-explicit = nodos solicitados
col-id = ID de nodo
col-long-name = Nombre largo
col-short-name = Corto
col-hardware = Hardware
col-last-heard = Última escucha
col-ago = Hace
never-heard = nunca
legend-own = nodo propio
legend-delete = a eliminar
legend-favorite = favorito
legend-ignored = ignorado
legend-unknown = parcialmente conocido

## Avisos

skip-not-found = { $id } no está en la nodedb.
skip-own = { $id } es el nodo propio del dispositivo y no se puede eliminar.
skip-favorite = { $id } es favorito y no se eliminará; desmárcalo antes.
skip-ignored = { $id } está ignorado y no se eliminará; al borrarlo se perdería la marca de ignorado.
note-protected = { $count ->
    [one] Se conserva { $count } nodo que cumple la selección por ser favorito o estar ignorado.
   *[other] Se conservan { $count } nodos que cumplen la selección por ser favoritos o estar ignorados.
}
note-never-heard = { $count ->
    [one] Se conserva { $count } nodo sin hora de última escucha.
   *[other] Se conservan { $count } nodos sin hora de última escucha.
} Probablemente el dispositivo no tenía la hora cuando los oyó. Usa --include-never para seleccionarlos también y --set-time para darle la hora actual al dispositivo.

## Eliminación

all-clean = ¡Todo limpio!
nothing-to-delete = No hay nada que eliminar.
dry-run = Simulación:
dry-run-summary = { $count ->
    [one] se eliminaría { $count } nodo. No se ha cambiado nada.
   *[other] se eliminarían { $count } nodos. No se ha cambiado nada.
}
confirm = { $count ->
    [one] ¿Eliminar { $count } nodo?
   *[other] ¿Eliminar { $count } nodos?
}
confirm-hint = [s/N]
# Respuestas aceptadas como «sí», separadas por comas (en minúsculas).
confirm-yes-answers = s, si, sí, y, yes
cancelled = Cancelado. No se ha eliminado ningún nodo.
removed = Eliminado { $id } ({ $name })
removed-unconfirmed = Enviada la eliminación de { $id } ({ $name }), pero sin confirmar
remove-failed = No se pudo eliminar { $id }: { $error }
result = { $total ->
    [one] { $removed } de { $total } nodo eliminado.
   *[other] { $removed } de { $total } nodos eliminados.
}
