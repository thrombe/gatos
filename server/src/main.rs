use std::{env, path::Path};

use iron::{Handler, Iron, IronResult, Request, Response};
use mount::Mount;
use staticfile::Static;

struct Mitm<T: Handler>(T);
impl<T: Handler> Handler for Mitm<T> {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        dbg!(&req);
        let resp = self.0.handle(req);
        dbg!(&resp);
        resp
    }
}

fn main() {
    let addr = "192.168.1.6:1337";
    println!("http://{addr}");

    let args = env::args().collect::<Vec<_>>();

    let mut mount = Mount::new();
    mount.mount(
        "/",
        Mitm(Static::new(Path::new(
            args.get(1).expect("no directory specified"),
        ))),
    );
    Iron::new(mount).http(addr).unwrap();
}
