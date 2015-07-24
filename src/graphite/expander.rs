use glob::glob;
use super::super::whisper::cache::Cache;
use std::path::Path;
use std::fs::PathExt;

pub struct NodeData;

pub enum QueryResultNode {
    WspNode(NodeData),
    DirNode(NodeData)
}

// A file-system only operation which can detect
// whisper files
pub fn expand(query: String, cache: &Cache) -> Vec<QueryResultNode> {
    let glob_pattern = dots_to_full_path_glob(query, cache);

    debug!("expanding {}", glob_pattern);
    
    let mut retval = vec![];
    let search = glob(&glob_pattern).unwrap();

    for search_result in search {
        match search_result {
            Ok(path_buf) => {
                debug!("expansion match: {:?}", path_buf);
                let path = path_buf.as_path();
                if path.is_dir() {
                    retval.push(QueryResultNode::DirNode(NodeData))
                } else {
                    retval.push(QueryResultNode::WspNode(NodeData))
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
fn dots_to_full_path_glob(query: String, cache: &Cache) -> String {
    let replaced = query.replace(".","/");

    let qualified_path = cache.base_path.join(replaced);
    let path : &Path = qualified_path.as_path();
    let str_rep = path.to_str().unwrap();
    let string_rep = str_rep.to_string();

    return string_rep;
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::super::super::whisper::cache::Cache;


    #[test]
    fn has_full_path(){
        let cache = Cache::new(Path::new("/tmp"));
        let input = "what.*.ever".to_string();
        let expected = "/tmp/what/*/ever";

        let full_glob = super::dots_to_full_path_glob(input, &cache);

        assert_eq!(full_glob, expected)
    }

    #[test]
    fn wont_go_up_directory(){
        let cache = Cache::new(Path::new("/tmp"));
        let input = "what.*.ever/../".to_string();
        let expected = "/tmp/what/*/ever////";

        let full_glob = super::dots_to_full_path_glob(input, &cache);

        assert_eq!(full_glob, expected)
    }

}
