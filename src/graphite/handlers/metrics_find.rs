use super::super::super::whisper::Cache;

use super::super::expander::expand;
use super::super::cache_holder::CacheHolder;
use super::super::error::StringError;

use iron::prelude::*;
use iron;
use urlencoded::UrlEncodedQuery;

use persistent::State;
use std::sync::{ Arc, RwLock };
use std::ops::DerefMut;

pub fn metrics_find(req: &mut Request) -> IronResult<Response> {
    let locked_cache : Arc< RwLock<Cache> > = req.get::<State<CacheHolder>>().unwrap();
    let mut cache_writer = locked_cache.write().unwrap();
    let mut cache = cache_writer.deref_mut();

    // Extract the decoded data as hashmap, using the UrlEncodedQuery plugin.
    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => {
            match hashmap.get("query") {
                Some(query) => {
                    if query.len() == 1 {
                        let ref first_query = query[0];
                        let http_body = do_find_metrics(first_query, &mut cache);
                        let mut http_res = Response::with((iron::status::Ok, http_body));

                        let jsony_ctype = iron::headers::ContentType(
                            iron::mime::Mime(
                                iron::mime::TopLevel::Application,
                                iron::mime::SubLevel::Json,
                                vec![(iron::mime::Attr::Charset, iron::mime::Value::Utf8)]
                            )
                        );
                        http_res.headers.set::<iron::headers::ContentType>(jsony_ctype);
                        Ok(http_res)
                    } else {
                        error!("must provide only 1 query string");
                        Err(IronError::new(StringError("Must provide only one query".to_string()), iron::status::BadRequest))
                    }
                },
                None => {
                    error!("no query was provided");
                    Err(IronError::new(StringError("Must provide query".to_string()), iron::status::BadRequest))
                }
            }
        },
        Err(_) => {
            error!("unknown error");
            Err(IronError::new(StringError("Error whoaaa".to_string()), iron::status::BadRequest))
        }
    }
}

fn do_find_metrics(query: &String, cache: &mut Cache) -> String {
    let hits = expand(query, cache);
    format!("[{}]", hits.join(","))
}