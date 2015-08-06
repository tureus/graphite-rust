use iron::prelude::*;
use iron;
use urlencoded::UrlEncodedQuery;

use std::io::Read;

// [{
//   "target": "entries",
//   "datapoints": [
//     [1.0, 1311836008],
//     [2.0, 1311836009],
//     [3.0, 1311836010],
//     [5.0, 1311836011],
//     [6.0, 1311836012]
//   ]
// }]
pub fn render(req: &mut Request) -> IronResult<Response> {
	let mut buf : Vec<u8> = Vec::new();
	let bytes_read = req.body.read_to_end(&mut buf);
	println!("body ({bytes_read:?}: {:#?}", buf, bytes_read=bytes_read);

    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => println!("Parsed GET request query string:\n {:?}", hashmap),
        Err(ref e) => println!("err {:?}", e)
    };

	Ok( Response::with( (iron::status::Ok, "hey".to_string() ) ) )
}

// target=hey.select%20metric&from=-6h&until=now&format=json&maxDataPoints=1425
// fn do_render() {

// }