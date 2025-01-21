/// Represents a unqiue item and all of the associated behaviors of such items.
pub trait Unqiue {
    fn id(&self) -> &String;
}
