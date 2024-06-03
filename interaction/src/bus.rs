pub trait Bus: Sized {
    fn from_usize(value: usize) -> Option<Self>;

    fn name(&self) -> &'static str;
}
