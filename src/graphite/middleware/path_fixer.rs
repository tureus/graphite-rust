use iron::prelude::*;
use iron;

pub struct PathFixer;

impl iron::middleware::BeforeMiddleware for PathFixer {
	fn before(&self, req: &mut Request) -> IronResult<()> {
		let ref mut path = req.url.path;
		if path.last().unwrap().len() == 0 {
			path.pop();
		}

		Ok(())
	}

	// fn catch(&self, _: &mut Request, err: IronError) -> IronResult<()> {
	// 	Err(err)
	// }
}