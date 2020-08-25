// Custum iterator transformers
pub trait IterExtras {
    fn leftovers<F, U>(self, f: F) -> Leftovers<Self, F, U>
    where
        Self: Sized + Iterator,
        F: FnMut(Self::Item) -> (U, Option<U>),
    {
        Leftovers::new(self, f)
    }
}

impl<T: ?Sized + Iterator> IterExtras for T {}

pub struct Leftovers<I: Iterator, F, U>
where
    F: FnMut(I::Item) -> (U, Option<U>),
{
    iter: I,
    f: F,
    stored: Option<U>,
}

impl<I: Iterator, F, U> Leftovers<I, F, U>
where
    F: FnMut(I::Item) -> (U, Option<U>),
{
    pub fn new(iter: I, f: F) -> Leftovers<I, F, U> {
        Leftovers {
            iter,
            f,
            stored: None,
        }
    }
}

impl<I: Iterator, F, U> Iterator for Leftovers<I, F, U>
where
    F: FnMut(I::Item) -> (U, Option<U>),
{
    type Item = U;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stored.is_some() {
            return self.stored.take();
        }

        let res = self.iter.next().map(&mut self.f);

        if let Some(tuple) = res {
            self.stored = tuple.1;
            Some(tuple.0)
        } else {
            None
        }
    }
}
