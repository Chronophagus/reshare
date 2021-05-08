pub trait NeqAssign {
    fn neq_assign(&mut self, rhs: Self) -> bool;
}

impl<T> NeqAssign for T
where
    T: PartialEq,
{
    fn neq_assign(&mut self, rhs: Self) -> bool {
        if *self == rhs {
            false
        } else {
            *self = rhs;
            true
        }
    }
}
