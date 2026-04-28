use mdns_sd::{ServiceDaemon, ServiceEvent};
fn main() {
    let mdns = ServiceDaemon::new().unwrap();
    mdns.shutdown().unwrap();
}
