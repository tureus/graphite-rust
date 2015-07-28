use super::config::Config;
use super::error::StringError;

use iron::prelude::*;
use iron;
use urlencoded::UrlEncodedQuery;
use persistent::State;

use router::Router;

use super::super::whisper::Cache;

// This is some weird voodoo so I can use persistent
// apparently I have to give a vtable which is used to resolve the
// type of the stored value so it can be recast "safely"
// but it adds one more level of indirection. Caveat emptor folks.
struct CacheHolder;
impl iron::typemap::Key for CacheHolder {
    // TODO: my brain hurts. what does this mean?
    type Value = Cache;
}

fn find_metrics(req: &mut Request) -> IronResult<Response> {
    // Extract the decoded data as hashmap, using the UrlEncodedQuery plugin.
    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => {
            info!("Parsed GET request query string:\n {:?}", hashmap);
            match hashmap.get("query") {
                Some(query) => {
                    if query.len() == 1 {
                        let ref first_query = query[0];
                        do_find_metrics(first_query)
                    } else {
                        Err(IronError::new(StringError("Must provide only one query".to_string()), iron::status::BadRequest))
                    }
                },
                None => {
                    Err(IronError::new(StringError("Must provide query".to_string()), iron::status::BadRequest))
                }
            }
        },
        Err(_) => {
            Err(IronError::new(StringError("Error whoaaa".to_string()), iron::status::BadRequest))
        }
    }
}

fn do_find_metrics(_: &String) -> IronResult<Response> {
    // expand(query);
    Ok(Response::with((iron::status::Ok, "Hello World\n")))
}

pub fn run(config: Config, cache: Cache) {
    let mut router = Router::new();
    router.get("/metrics/find", find_metrics);
    router.get("/metrics/find/", find_metrics);

    let mut chain = Chain::new(router);
    chain.link( State::<CacheHolder>::both(cache) );

    Iron::new(chain).http(config.bind_spec).unwrap(); 
}

// TODO: not sure how to test url encoded query middleware :(
// #[cfg(test)]
// mod tests {
//     // extern crate iron-test;
//     // use iron::Iron;
//     // use super::find_metrics;

//     // #[test]
//     // fn test_stuff(){
//     //     let req = request::new(method::Get, "localhost:3000");
//     // }
// }
