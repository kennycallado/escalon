# Escalon

**Warning**:
While passing the count of jobs (via `set_count(move || {})`) instead of a vector of jobs, we rely on the upper layer to utilize a database, but our choices are constrained by the UDP buffer size.

## TODO:
- [ ] colisión:
  Existe la posibiliad de asignar tareas de un muerto a un cliente que a muerto en
  el periodo de espera... debería vovler a comprobar después de la espera...

## Implementation tests:
- docker compose up
  - then kill one
