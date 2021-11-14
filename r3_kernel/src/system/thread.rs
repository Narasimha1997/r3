

#[derive(Clone, Debug)]
pub struct ThreadID(u64);

impl ThreadID {
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    } 
}