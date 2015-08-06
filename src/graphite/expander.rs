use glob::glob;
use super::super::whisper::cache::Cache;
use std::path::{ Path, PathBuf };
use std::fs::{ PathExt, read_dir };
// use std::collections::BTreeMap;
// use rustc_serialize::json::{self, ToJson, Json};

#[derive(Debug)]
pub enum QueryResultNode {
    WspNode(PathBuf, PathBuf),
    // path to dir, has children, 
    DirNode(PathBuf, PathBuf, bool, )
}

const EXPANDABLE : i8 = 1;
const NOT_EXPANDABLE : i8 = 2;

fn text_and_file_name<'a>(cache_root: &'a Path, path: &'a Path) -> (&'a str,String) {
    let mut text = path.file_name().unwrap().to_str().unwrap();
    text = &text[0..text.len()-4];

    let cache_root_str : &str = &cache_root.to_str().unwrap();
    let path_str : &str = &path.to_str().unwrap();

    // Subtract the common cache path bits. And trim off the .wsp while we're at it.
    // One of the most reckless code path in here! 8-)
    let without_root : &str = &path_str[ cache_root_str.len()+1 .. path_str.len()-4 ];
    let mut metric_name = without_root.to_string();
    metric_name = metric_name.replace("/",".");

    return (text, metric_name)
}

fn text_and_folder_name<'a>(cache_root: &'a Path, path: &'a Path) -> (&'a str,String) {
    let text = path.file_name().unwrap().to_str().unwrap();

    let cache_root_str : &str = &cache_root.to_str().unwrap();
    let path_str : &str = &path.to_str().unwrap();

    // Subtract the common cache path bits. And trim off the .wsp while we're at it.
    // One of the most reckless code path in here! 8-)
    let without_root : &str = &path_str[ cache_root_str.len()+1 .. path_str.len() ];
    let mut metric_name = without_root.to_string();
    metric_name = metric_name.replace("/",".");

    return (text, metric_name)
}


impl QueryResultNode {
    fn to_json(&self) -> String {
        match *self {
            QueryResultNode::WspNode(ref cache_root, ref path) => {
                let pair = text_and_file_name(cache_root, path);

                format!(r#"{{"leaf": {leaf}, "context": {{}}, "text": "{text}", "expandable": {expandable}, "id": "{id}", "allowChildren": {allow_children}}}"#,
                            allow_children=0, expandable=0, leaf=0,
                            text=pair.0, id=pair.1)
            },
            QueryResultNode::DirNode(ref cache_root, ref path, ref has_children) => {
                let pair = text_and_folder_name(cache_root, path);

                format!(r#"{{"leaf": {leaf}, "context": {{}}, "text": "{text}", "expandable": {expandable}, "id": "{id}", "allowChildren": {allow_children}}}"#,
                            allow_children=0, expandable= if *has_children { EXPANDABLE } else { NOT_EXPANDABLE } , leaf=0,
                            text=pair.0, id=pair.1)
            }
        }
    }
}

// A file-system only operation which can detect
// whisper files
pub fn expand(query: &String, cache: &Cache) -> Vec<String> {
    let glob_pattern = dots_to_full_path_glob(query, cache);

    debug!("expanding {}", glob_pattern);
    
    let mut retval = vec![];
    let search = glob(&glob_pattern).unwrap();

    for search_result in search {
        match search_result {
            Ok(path_buf) => {
                debug!("expansion match: {:?}", path_buf);
                let is_dir = {
                    let path = path_buf.as_path();
                    path.is_dir()
                };

                let has_children = if is_dir {                    
                    read_dir( &path_buf ).unwrap().any(|f| {
                        let metadata = f.unwrap().metadata().unwrap();
                        metadata.is_dir() || metadata.is_file()
                    })
                } else {
                    false
                };

                if is_dir {
                    retval.push( QueryResultNode::DirNode( cache.base_path.clone(), path_buf, has_children).to_json() )
                } else {
                    retval.push( QueryResultNode::WspNode( cache.base_path.clone(), path_buf).to_json() )
                }
            },
            Err(e) => {
                info!("error in search: {:?}", e)
            }
        }
    }

    debug!("retval len: {}", retval.len());

    return retval
}

// TODO: is it really this much work?
// TODO: what about security concerns for traversing the file system? Can you craft a query such that ".." shows up? (Don't think so)
fn dots_to_full_path_glob(query: &String, cache: &Cache) -> String {
    let replaced = query.replace(".","/");

    let qualified_path = cache.base_path.join(replaced);
    let path : &Path = qualified_path.as_path();
    let str_rep = path.to_str().unwrap();
    let string_rep = str_rep.to_string();
    // string_rep.push_str(".wsp");

    return string_rep;
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::super::super::whisper::cache::Cache;
    use super::QueryResultNode;

    #[test]
    fn has_full_path(){
        let cache = Cache::new(Path::new("/tmp"));
        let input = "what.*.ever".to_string();
        let expected = "/tmp/what/*/ever";

        let full_glob = super::dots_to_full_path_glob(&input, &cache);

        assert_eq!(full_glob, expected)
    }

    // Not trying that hard but a simple sanity check
    #[test]
    fn wont_go_up_directory(){
        let cache = Cache::new(Path::new("/tmp"));
        let input = "what.*.ever/../".to_string();
        let expected = "/tmp/what/*/ever////";

        let full_glob = super::dots_to_full_path_glob(&input, &cache);

        assert_eq!(full_glob, expected)
    }

    #[test]
    fn wsp_node_json(){
        let root = Path::new("/tmp/thing").to_path_buf();
        let deep = Path::new("/tmp/thing/is/cool/bear.wsp").to_path_buf();
        let wsp_node = QueryResultNode::WspNode( root, deep );
        let expected = "{\"leaf\": 0, \"context\": {}, \"text\": \"bear\", \"expandable\": 0, \"id\": \"is.cool.bear\", \"allowChildren\": 0}";
        assert_eq!( wsp_node.to_json(), expected.to_string() )
    }
}
