//
// TODO:
// - [ ] Implementar un sistema de logs
// - [ ] Implementar un sistema de errores
// - [ ] Implementar un sistema de tests
// - [ ] Implementar un sistema de configuración
// - [ ] Implementar un sistema de comandos
//
// - [ ] Implementar un sistema de stados
//   - Cada instancia maneja número de jobs
//   - Cada instancia maneja memoria en uso
//   - Determinar que cliente maneja los jobs del cliente caido
//

//
// echo $RANDOM | xargs -I[] echo '{ "action": { "Join": "'[]'" } }' | socat - udp-datagram:192.168.1.255:65056,broadcast
// rand=$(echo $RANDOM); while true;do sleep 5; echo '{ "action": { "Check": "'$rand'" } }' | socat - udp-datagram:192.168.1.255:65056,broadcast ;done
// Check also add new clients
//

use anyhow::Result;
use sysinfo::{System, SystemExt};

mod constants;
mod server;
mod types;

use crate::server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    let mut sys = System::new();
    sys.refresh_all();

    // println!("Memory: {} Mb", sys.used_memory() / 1024 / 1024);
    // println!("Hostname: {}", sys.host_name().unwrap());

    let port = std::env::var("PORT").unwrap_or("65056".to_string());
    let addr = std::env::var("ADDR").unwrap_or("0.0.0.0".to_string());

    let mut server = Server::new(addr, port, sys.host_name().unwrap()).await?;
    server.listen(|id, addr, port| { println!("Server {} listening: {}:{}", id, addr, port) }).await?;

    // puede que en capas superiores necesite
    // guardar en db el id u otros...

    Ok(())
}
