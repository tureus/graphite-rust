use super::config::Config;
use super::error::StringError;

use iron::prelude::*;
use iron;
use urlencoded::UrlEncodedQuery;

use router::Router;

fn find_metrics(req: &mut Request) -> IronResult<Response> {
    // Extract the decoded data as hashmap, using the UrlEncodedQuery plugin.
    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => {
            println!("Parsed GET request query string:\n {:?}", hashmap);
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
            Err(IronError::new(StringError("Error".to_string()), iron::status::Ok))
        }
    }
}

fn do_find_metrics(_: &String) -> IronResult<Response> {
    // expand(query);
    Ok(Response::with((iron::status::Ok, "Hello World\n")))
}

pub fn run(config: Config) {
    let mut router = Router::new();
    router.get("/metrics/find", find_metrics);
    router.get("/metrics/find/", find_metrics);
    Iron::new(router).http(config.bind_spec).unwrap(); 
}
