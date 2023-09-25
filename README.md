# Escalon

## notes:

### listening

listen udp
``` bash
nc -ul 65056
```

### sending

should be send from a container...

send join upd
``` bash
echo -n "{\"action\":{\"Join\":{\"sender_id\":\"TESTER\",\"start_time\":{\"secs_since_epoch\":1695629468,\"nanos_since_epoch\":870772893}}}}" | nc -u -q1 0.0.0.0 65056
```

send check udp
``` bash
echo -n "{\"action\":{\"Check\":{\"sender_id\":\"TESTER\",\"jobs\":0}}}" | nc -u -q1 0.0.0.0 65056
```

## TODO:
- [ ] colisión:
  Existe la posibiliad de asignar tareas de un muerto a un cliente que a muerto en
  el periodo de espera... debería vovler a comprobar después de la espera...

## Implementation tests:
- docker compose up
  - then kill one
