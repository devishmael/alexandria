---
id: ADR-004-single-writer-mpsc-storage
title: "ADR-004: Persistencia SQLite WAL mediante Actor Single-Writer MPSC con Confirmación Post-Commit"
type: decision
status: Aceptado
version: "2.4.0-OmniPragmatic"
created_at: 2026-10-19
tags:
  - adr
  - sqlite
  - wal
  - mpsc
  - concurrency
  - fts5
---

# ADR-004: Persistencia SQLite WAL mediante Actor Single-Writer MPSC con Confirmación Post-Commit

## 1. Estado
**Aceptado** — Vigente y de cumplimiento obligatorio en todo el runtime.

---

## 2. Contexto

El runtime de Alexandria requiere almacenar localmente en `~/.alexandria/vault.db` el catálogo de habilidades, el estado de los *Circuit Breakers*, los incidentes forenses (`FAIL-XXX`) y los índices léxicos FTS5. En un entorno asíncrono (Tokio) con workers paralelos y recargas del *File Watcher*, el uso ingenuo de SQLite presenta tres riesgos críticos:

1. **Contención de Escritura (`SQLITE_BUSY` / `SQLITE_LOCKED`):** Aunque SQLite en modo WAL permite múltiples lectores simultáneos con un escritor, solo admite **un único escritor activo a la vez**. Abrir múltiples conexiones de escritura desde distintas tareas de Tokio provoca errores de bloqueo bajo ráfagas concurrentes.
2. **Violación de Consistencia *Read-Your-Writes* por Notificación Prematura:** En implementaciones basadas en actores con agrupamiento de lotes (*batching*), si el actor envía el acuse de recibo (`oneshot::Sender::send`) tras ejecutar el `INSERT` en memoria pero **antes** de ejecutar `tx.commit()`, el hilo solicitante puede realizar inmediatamente una lectura en otra conexión y no encontrar el registro recién insertado.
3. **Corrupción de Tablas Virtuales FTS5 (`SQLITE_CORRUPT_VTAB`) y Errores de Sintaxis:** El uso de tablas FTS5 con contenido externo (`content='...'`) sin el conjunto completo de tres triggers (`INSERT`, `DELETE`, `UPDATE`), o el envío directo de stack traces crudos con caracteres especiales (`:`, `\`, `"`, `AND`, `OR`) a la cláusula `MATCH`, corrompe el índice o aborta las consultas de recuperación forense.

---

## 3. Decisión

Adoptamos el patrón **Single-Writer MPSC Batching Actor en Hilo Dedicado de OS con Confirmación Post-Commit**, implementado en [[database-schema]]:

1. **Hilo Dedicado de OS (`alexandria-sqlite-writer`):**
   - Se instancia un único hilo nativo mediante `std::thread::Builder` que posee en exclusividad la única conexión de lectura/escritura hacia `vault.db`.
   - Todas las tareas asíncronas de Tokio envían comandos `VaultWriteCommand` a través de un canal acotado `tokio::sync::mpsc::channel(512)`.
2. **Agrupamiento Transaccional por Lotes (`try_recv` Batching):**
   - El hilo escritor espera el primer comando con `blocking_recv()` y drena de inmediato hasta 63 comandos adicionales disponibles en la cola con `try_recv()`, ejecutando hasta 64 operaciones dentro de una sola transacción `conn.transaction()`.
3. **Notificación Estrictamente Post-Commit:**
   - La función `stage_command_in_tx` ejecuta las sentencias SQL y devuelve un *closure* diferido `Box<dyn FnOnce(Result<(), String>)>`.
   - Los canales `oneshot` de respuesta se invocan **estrictamente después** de que `tx.commit()` ha confirmado la escritura en el archivo WAL.
4. **Blindaje FTS5 Simétrico y Sanitizador Disyuntivo:**
   - Toda tabla FTS5 de contenido externo declara obligatoriamente sus tres triggers (`_ai`, `_ad`, `_au`).
   - Todo stack trace se procesa mediante `build_safe_fts5_or_query`, filtrando palabras reservadas de FTS5 y construyendo una consulta segura `"tok1" OR "tok2"` ordenada por `bm25()`.

---

## 4. Consecuencias e Invariantes

### Consecuencias Positivas
- **Cero `SQLITE_BUSY` por Diseño:** Al existir un único descriptor de escritura serializado en memoria, es matemáticamente imposible sufrir colisiones entre escritores.
- **Rendimiento Multiplicado en Ráfagas:** El *batching* de hasta 64 comandos reduce las operaciones de `fsync`/WAL en un orden de magnitud.
- **Consistencia Fuerte *Read-Your-Writes*:** En el instante en que un `.await` sobre el `oneshot` retorna `Ok`, cualquier conexión `SQLITE_OPEN_READ_ONLY` en cualquier hilo observa inmediatamente los datos comprometidos.

### Compromisos y Costos Asumidos
- Ligera complejidad adicional en el diseño de comandos asíncronos mediante emparejamiento de canales `mpsc` + `oneshot`.

### Invariantes Constitucionales
- **Invariante S1:** Queda prohibido abrir conexiones SQLite con permisos de escritura fuera del hilo `alexandria-sqlite-writer`.
- **Invariante S2:** Ningún comando `VaultWriteCommand` puede enviar su respuesta `oneshot` antes de la invocación exitosa a `tx.commit()`.

---

## 5. Enlaces y Relaciones

- **Mapa Maestro:** [[index]]
- **Topología General:** [[overview]]
- **Implementación DDL y Actor:** [[database-schema]]
- **Runtime Central:** [[core-engine]]
- **Registro Forense:** [[failures]]
- **Decisiones Relacionadas:** [[ADR-001-kernel-space-harness]], [[ADR-002-prefix-pinned-caching]]
