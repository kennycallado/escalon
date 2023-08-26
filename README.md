
## TODO:
- [ ] Determinate memory usage

## Implementation tests:
- docker compose up
  - then kill one
- by shell
``` bash
echo $RANDOM | xargs -I[] echo '{ "action": { "Join": "'[]'" } }' | socat - udp-datagram:192.168.1.255:65056,broadcast
# or
rand=$(echo $RANDOM); while true;do sleep 5; echo '{ "action": { "Check": "'$rand'" } }' | socat - udp-datagram:192.168.1.255:65056,broadcast ;done
```