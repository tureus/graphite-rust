use iron;
use super::super::whisper::Cache;

// This is some weird voodoo so I can use persistent
// apparently I have to give a vtable which is used to resolve the
// type of the stored value so it can be recast "safely"
// but it adds one more level of indirection. Caveat emptor folks.
pub struct CacheHolder;
impl iron::typemap::Key for CacheHolder {
    // TODO: my brain hurts. what does this mean?
    type Value = Cache;
}