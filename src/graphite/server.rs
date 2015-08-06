use super::config::Config;
use super::middleware::PathFixer;

use iron::prelude::*;
use persistent::State;

use router::Router;

use super::super::whisper::Cache;
use super::handlers;
use super::cache_holder::CacheHolder;

pub fn run(config: Config, cache: Cache) {
    let mut router = Router::new();
    router.get("/metrics/find", handlers::metrics_find);
    router.post("/render", handlers::render);

    let mut chain = Chain::new(router);
    chain.link_before(PathFixer);
    chain.link( State::<CacheHolder>::both(cache) );

    Iron::new(chain).http(config.bind_spec).unwrap(); 
}
